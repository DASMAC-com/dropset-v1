use std::{
    str::FromStr,
    sync::LazyLock,
};

use dropset_interface::state::SYSTEM_PROGRAM_ID;
use regex::Regex;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::UiTransactionStatusMeta;

#[derive(Clone, Debug)]
pub struct InnerInstructionCompute {
    pub program_id: Pubkey,
    pub units_consumed: u64,
    pub total_consumption: u64,
    pub invocation_index: u8,
    pub stack_height: u8,
    // The absolute order of the `invoke` and `consumed X of Y compute units` emissions in logs.
    pub absolute_invoke_index: u8,
    pub absolute_cu_index: u8,
}

/// Example: `Program TokenkegQ...s623VQ5DA consumed 1405 of 379808 compute units`
static COMPUTE_LOG_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Program (\S+) consumed (\d+) of (\d+) compute units").unwrap());

/// Example: `Program TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q invoke [1]`
static INVOKE_INDEX_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Program (\S+) invoke \[(\d+)\]").unwrap());

/// Example: `Program TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q success`
static INVOKE_SUCCESS_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"Program (\S+) success").unwrap());

pub fn parse_logs_for_compute(meta: &UiTransactionStatusMeta) -> Vec<InnerInstructionCompute> {
    let mut res = vec![];
    let mut stack = ComputeBuilder::default();

    meta.log_messages
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .for_each(|log| {
            if let Some((program_id, stack_height)) = parse_invoke(log) {
                stack.push_new(program_id);
                debug_assert_eq!(stack_height as usize, stack.stack_height());
            } else if let Some((program_id, used, total)) = parse_compute(log) {
                stack.push_compute_info(&program_id, used, total);
            } else if let Some(program_id) = parse_success(log) {
                let built = stack.build_success(&program_id);
                res.push(built);
            }
        });

    res
}

#[inline]
fn parse_compute(log: &str) -> Option<(Pubkey, u64, u64)> {
    COMPUTE_LOG_PATTERN.captures(log).map(|cap| {
        let program = Pubkey::from_str(&cap[1]).expect("Should be a pubkey");
        let used: u64 = cap[2]
            .parse()
            .expect("Should parse compute used as a number");
        let total: u64 = cap[3]
            .parse()
            .expect("Should parse total compute used as a number");
        (program, used, total)
    })
}

#[inline]
fn parse_invoke(log: &str) -> Option<(Pubkey, u8)> {
    INVOKE_INDEX_PATTERN.captures(log).map(|cap| {
        let program = Pubkey::from_str(&cap[1]).expect("Should be a pubkey");
        let index: u8 = cap[2].parse().expect("Should parse number");

        (program, index)
    })
}

#[inline]
fn parse_success(log: &str) -> Option<Pubkey> {
    INVOKE_SUCCESS_PATTERN
        .captures(log)
        .map(|cap| Pubkey::from_str(&cap[1]).expect("Should be a pubkey"))
}

struct ComputeInfo {
    /// The instruction index.
    index: u8,
    /// The program's ID.
    program_id: Pubkey,
    /// The height of the invocation/call stack.
    stack_height: u8,
    /// The compute units consumed.
    units_consumed: Option<u64>,
    /// The total compute consumption thus far.
    total_consumption: Option<u64>,
    /// The absolute index of the invoke. This is interpolated with the stack height-aware index.
    absolute_invoke_index: u8,
}

#[derive(Default)]
struct ComputeBuilder {
    invocation_index: u8,
    absolute_index: u8,
    stack: Vec<ComputeInfo>,
}

impl ComputeBuilder {
    fn stack_height(&self) -> usize {
        let height = self.stack.len();
        debug_assert!(height < u8::MAX as usize);
        height
    }

    fn push_new(&mut self, program_id: Pubkey) {
        self.invocation_index += 1;
        self.absolute_index += 1;

        self.stack.push(ComputeInfo {
            index: self.invocation_index,
            program_id,
            // Top level instructions start at a height of 1.
            stack_height: (self.stack_height() + 1) as u8,
            units_consumed: None,
            total_consumption: None,
            absolute_invoke_index: self.absolute_index,
        });
    }

    fn push_compute_info(
        &mut self,
        program_id: &Pubkey,
        units_consumed: u64,
        total_consumption: u64,
    ) {
        let top = self
            .stack
            .last_mut()
            .expect("Should never encounter CU with no head");
        debug_assert_eq!(&top.program_id, program_id);
        debug_assert_eq!(top.units_consumed, None);
        debug_assert_eq!(top.total_consumption, None);
        top.units_consumed.replace(units_consumed);
        top.total_consumption.replace(total_consumption);
    }

    fn build_success(&mut self, program_id_check: &Pubkey) -> InnerInstructionCompute {
        self.absolute_index += 1;

        let top = self
            .stack
            .pop()
            .expect("Stack shouldn't be empty on success");
        let ComputeInfo {
            index,
            program_id,
            stack_height,
            units_consumed,
            total_consumption,
            absolute_invoke_index,
        } = top;
        debug_assert_eq!(program_id_check, &program_id);
        debug_assert!(units_consumed.is_some() || program_id == SYSTEM_PROGRAM_ID.into());
        debug_assert!(total_consumption.is_some() || program_id == SYSTEM_PROGRAM_ID.into());
        debug_assert!(absolute_invoke_index != self.absolute_index);
        InnerInstructionCompute {
            program_id,
            units_consumed: units_consumed.unwrap_or_default(),
            total_consumption: total_consumption.unwrap_or_default(),
            invocation_index: index,
            stack_height,
            absolute_invoke_index,
            absolute_cu_index: self.absolute_index,
        }
    }
}
