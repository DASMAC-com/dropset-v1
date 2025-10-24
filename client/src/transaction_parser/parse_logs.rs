use std::{
    str::FromStr,
    sync::LazyLock,
};

use anyhow::ensure;
use dropset_interface::state::SYSTEM_PROGRAM_ID;
use itertools::Itertools;
use regex::Regex;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::UiTransactionStatusMeta;

use crate::transaction_parser::ParsedOuterInstruction;

#[derive(Debug, Clone)]
pub struct ComputeInfo {
    /// The absolute instruction invocation index.
    pub absolute_invocation_index: usize,
    /// The program's ID.
    pub program_id: Pubkey,
    /// The height of the invocation/call stack.
    pub stack_height: usize,
    /// The compute units consumed.
    pub units_consumed: Option<u64>,
    /// The total compute consumption thus far.
    pub total_consumption: Option<u64>,
    /// The outer/parent index, if this is an inner instruction.
    pub parent_index: Option<usize>,
}

pub struct GroupedComputeInfo {
    pub parent: ComputeInfo,
    pub children: Vec<ComputeInfo>,
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

type ParseResult<T> = Result<T, anyhow::Error>;

pub fn parse_logs_for_compute(
    meta: &UiTransactionStatusMeta,
) -> ParseResult<Vec<GroupedComputeInfo>> {
    let mut stack = ComputeBuilder::default();

    for log in meta.log_messages.as_ref().unwrap_or(&vec![]) {
        if let Some((program_id, expected_height)) = parse_invoke(log) {
            stack.push_new(program_id);

            let heights_match = expected_height == stack.stack.last().unwrap().stack_height;
            ensure!(heights_match, "Stack height mismatch");
        } else if let Some((program_id, used, total)) = parse_compute(log) {
            stack.push_compute_info(&program_id, used, total)?;
        } else if let Some(program_id) = parse_success(log) {
            stack.push_success(&program_id)?;
        }
    }

    stack.build_compute_infos()
}

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
fn parse_invoke(log: &str) -> Option<(Pubkey, usize)> {
    INVOKE_INDEX_PATTERN.captures(log).map(|cap| {
        let program = Pubkey::from_str(&cap[1]).expect("Should be a pubkey");
        let index: usize = cap[2].parse().expect("Should parse number");

        (program, index)
    })
}

#[inline]
fn parse_success(log: &str) -> Option<Pubkey> {
    INVOKE_SUCCESS_PATTERN
        .captures(log)
        .map(|cap| Pubkey::from_str(&cap[1]).expect("Should be a pubkey"))
}

#[derive(Default)]
struct ComputeBuilder {
    absolute_invocation_index: usize,
    scope_index: usize,
    stack: Vec<ComputeInfo>,
    // The absolute invocation index of each parent instruction.
    parents: Vec<usize>,
    /// The absolutely ordered collection of all successfully invoked instructions.
    infos: Vec<ComputeInfo>,
}

impl ComputeBuilder {
    pub fn stack_height(&self) -> usize {
        // Stack height is 1-indexed in the Solana SDK.
        self.stack.len() + 1
    }

    fn push_new(&mut self, program_id: Pubkey) {
        let is_parent = self.stack.is_empty();
        let parent_index = (!is_parent).then(|| self.parents.len() - 1);

        if is_parent {
            self.parents.push(self.absolute_invocation_index);
        }

        let info = ComputeInfo {
            absolute_invocation_index: self.absolute_invocation_index,
            program_id,
            stack_height: self.stack_height(),
            units_consumed: None,
            total_consumption: None,
            parent_index,
        };

        self.absolute_invocation_index += 1;
        self.scope_index += 1;

        self.stack.push(info);
    }

    fn push_compute_info(
        &mut self,
        program_id: &Pubkey,
        units_consumed: u64,
        total_consumption: u64,
    ) -> ParseResult<()> {
        let top = self
            .stack
            .last_mut()
            .ok_or(anyhow::Error::msg("Should never encounter CU with no head"))?;

        ensure!(&top.program_id == program_id, "Stack depth mismatch");
        ensure!(&top.program_id == program_id, "Units consumed != None");
        ensure!(&top.program_id == program_id, "Total consumption != None");
        top.units_consumed.replace(units_consumed);
        top.total_consumption.replace(total_consumption);

        Ok(())
    }

    fn push_success(&mut self, program_id: &Pubkey) -> ParseResult<()> {
        self.scope_index += 1;

        let info = self
            .stack
            .pop()
            .ok_or(anyhow::Error::msg("Stack shouldn't be empty on success"))?;

        ensure!(program_id == &info.program_id, "Stack depth mismatch");
        let no_cu_expected = info.program_id == SYSTEM_PROGRAM_ID.into();
        let valid_consumed = no_cu_expected || info.units_consumed.is_some();
        let valid_total = no_cu_expected || info.total_consumption.is_some();
        ensure!(valid_consumed, "Missing units consumed");
        ensure!(valid_total, "Missing total consumption");

        self.infos.push(info);

        Ok(())
    }

    pub fn build_compute_infos(self) -> ParseResult<Vec<GroupedComputeInfo>> {
        let ComputeBuilder {
            stack,
            parents,
            infos,
            ..
        } = self;
        ensure!(stack.is_empty(), "Stack isn't empty on build()");

        // `infos` is currently in completion order. Sort it in invocation order and partition by
        // parent/child.
        let (parent_infos, children_infos): (Vec<_>, Vec<_>) = infos
            .into_iter()
            .sorted_by_key(|info| info.absolute_invocation_index)
            .partition(|info| info.parent_index.is_none());

        let mut parent_instructions = parent_infos
            .into_iter()
            .map(|info| GroupedComputeInfo {
                parent: info,
                children: vec![],
            })
            .collect_vec();

        ensure!(
            parents.len() == parent_instructions.len(),
            "Parent length mismatch"
        );

        for child in children_infos.into_iter() {
            ensure!(
                child.parent_index.is_some(),
                "Child should have parent index"
            );

            let parent_idx = child.parent_index.unwrap();
            if parent_instructions.get_mut(parent_idx).is_none() {
                println!(
                    "parent index: {parent_idx}, {child:#?}, LEN: {}",
                    parent_instructions.len()
                );
            }

            parent_instructions
                .get_mut(parent_idx)
                .expect("Parent should exist")
                .children
                .push(child);
        }

        Ok(parent_instructions)
    }
}

pub fn add_infos_to_outer_instructions(
    outers: &mut [ParsedOuterInstruction],
    compute_map: Vec<GroupedComputeInfo>,
) -> ParseResult<()> {
    for (i, group) in compute_map.into_iter().enumerate() {
        let parsed = outers
            .get_mut(i)
            .ok_or(anyhow::Error::msg("Matching parent should exist"))?;

        let not_mismatched = parsed.outer_instruction.program_id == group.parent.program_id;
        ensure!(not_mismatched, "Mismatched parent program IDs");

        let valid_children_lengths = parsed.inner_instructions.len() == group.children.len();
        ensure!(valid_children_lengths, "Mismatched children length");

        parsed.outer_instruction.compute_info.replace(group.parent);

        let grouped_inners = group.children.into_iter();
        for (inner, cu) in parsed.inner_instructions.iter_mut().zip(grouped_inners) {
            let matching_program_ids = inner.program_id == cu.program_id;
            ensure!(matching_program_ids, "Mismatched child program IDs");

            inner.compute_info.replace(cu);
        }
    }

    Ok(())
}
