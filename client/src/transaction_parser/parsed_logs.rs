//! Parses compute and program logs into structured per-instruction compute usage and log metadata.

use std::str::FromStr;

use anyhow::ensure;
use dropset_interface::state::SYSTEM_PROGRAM_ID;
use itertools::Itertools;
use lazy_regex::*;
use solana_sdk::pubkey::Pubkey;

use crate::{
    transaction_parser::ParsedOuterInstruction,
    COMPUTE_BUDGET_ID,
};

/// A struct that represents information extracted from a transaction's parsed program logs.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub struct ParsedLogs {
    /// The instruction invocation index- i.e., the order in which the instruction was executed.
    pub invocation_index: usize,
    /// The program's ID.
    pub program_id: Pubkey,
    /// The height of the invocation/call stack.
    pub stack_height: usize,
    /// The compute units consumed.
    pub units_consumed: Option<u64>,
    /// The compute consumption allowance thus far.
    pub consumption_allowance: Option<u64>,
    /// The outer/parent index, if this is an inner instruction.
    pub parent_index: Option<usize>,
    /// The program's explicit log messages; i.e., the log messages that start with "Program log:".
    pub program_logs: Vec<String>,
}

pub struct GroupedParsedLogs {
    pub parent: ParsedLogs,
    pub children: Vec<ParsedLogs>,
}

/// Example: `Program TokenkegQ...s623VQ5DA consumed 1405 of 379808 compute units`
static COMPUTE_LOG_PATTERN: &lazy_regex::Lazy<lazy_regex::Regex> =
    regex!(r"Program (\S+) consumed (\d+) of (\d+) compute units");

/// Example: `Program TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q invoke [1]`
static INVOKE_WITH_HEIGHT_PATTERN: &lazy_regex::Lazy<lazy_regex::Regex> =
    regex!(r"Program (\S+) invoke \[(\d+)\]");

/// Example: `Program TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q success`
static INVOKE_SUCCESS_PATTERN: &lazy_regex::Lazy<lazy_regex::Regex> =
    regex!(r"Program (\S+) success");

/// Example: `Program log: hello world!`
static PROGRAM_LOG_PATTERN: &lazy_regex::Lazy<lazy_regex::Regex> = regex!(r"Program log: (.*)\z");

type ParseResult<T> = Result<T, anyhow::Error>;

/// Parses program logs for compute usage information. This function iteratively builds upon
/// received log messages to construct a complete mapping of each instruction's chronological
/// invocation index, stack height, and compute usage.
///
/// This also facilitates grouping outer/parent instructions with their inner/child instructions.
pub fn parse_logs_for_compute(log_messages: &[String]) -> ParseResult<Vec<GroupedParsedLogs>> {
    let mut stack = ComputeBuilder::default();

    for log in log_messages {
        let trimmed = log.trim();

        if let Some((program_id, expected_height)) = parse_invoke(trimmed) {
            stack.push_new(program_id);

            let heights_match = expected_height == stack.stack.last().unwrap().stack_height;
            ensure!(heights_match, "Stack height mismatch");
        } else if let Some((program_id, used, allowed)) = parse_compute(trimmed) {
            stack.push_compute_info(&program_id, used, allowed)?;
        } else if let Some(program_id) = parse_success(trimmed) {
            stack.push_success(&program_id)?;
        } else if let Some(program_log) = parse_program_log(trimmed) {
            stack.push_program_log(program_log)?;
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
        let allowance: u64 = cap[3]
            .parse()
            .expect("Should parse compute allowance as a number");
        (program, used, allowance)
    })
}

#[inline]
fn parse_invoke(log: &str) -> Option<(Pubkey, usize)> {
    INVOKE_WITH_HEIGHT_PATTERN.captures(log).map(|cap| {
        let program = Pubkey::from_str(&cap[1]).expect("Should be a pubkey");
        let stack_height: usize = cap[2].parse().expect("Should parse number");

        (program, stack_height)
    })
}

#[inline]
fn parse_success(log: &str) -> Option<Pubkey> {
    INVOKE_SUCCESS_PATTERN
        .captures(log)
        .map(|cap| Pubkey::from_str(&cap[1]).expect("Should be a pubkey"))
}

#[inline]
fn parse_program_log(log: &str) -> Option<String> {
    PROGRAM_LOG_PATTERN
        .captures(log)
        .map(|cap| cap[1].to_string())
}

#[derive(Default)]
struct ComputeBuilder {
    invocation_index: usize,
    scope_index: usize,
    stack: Vec<ParsedLogs>,
    /// All parent instructions' invocation index. This is inherently chronologically sorted.
    /// This facilitates tracking the stack height/depth while traversing logs.
    parents: Vec<usize>,
    /// The chronologically ordered collection of all successfully invoked instructions.
    infos: Vec<ParsedLogs>,
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
            self.parents.push(self.invocation_index);
        }

        let info = ParsedLogs {
            invocation_index: self.invocation_index,
            program_id,
            stack_height: self.stack_height(),
            units_consumed: None,
            consumption_allowance: None,
            parent_index,
            program_logs: vec![],
        };

        self.invocation_index += 1;
        self.scope_index += 1;

        self.stack.push(info);
    }

    fn push_compute_info(
        &mut self,
        program_id: &Pubkey,
        units_consumed: u64,
        consumption_allowance: u64,
    ) -> ParseResult<()> {
        let top = self
            .stack
            .last_mut()
            .ok_or(anyhow::Error::msg("Shouldn't encounter CU with no head"))?;

        ensure!(&top.program_id == program_id, "Stack depth mismatch");
        ensure!(&top.program_id == program_id, "Units consumed != None");
        ensure!(&top.program_id == program_id, "Consumption allowed != None");
        top.units_consumed.replace(units_consumed);
        top.consumption_allowance.replace(consumption_allowance);

        Ok(())
    }

    fn push_program_log(&mut self, log: String) -> ParseResult<()> {
        let top = self
            .stack
            .last_mut()
            .ok_or(anyhow::Error::msg("Shouldn't encounter a log with no head"))?;

        top.program_logs.push(log);

        Ok(())
    }

    fn push_success(&mut self, program_id: &Pubkey) -> ParseResult<()> {
        self.scope_index += 1;

        let info = self
            .stack
            .pop()
            .ok_or(anyhow::Error::msg("Stack shouldn't be empty on success"))?;

        ensure!(program_id == &info.program_id, "Stack depth mismatch");
        let no_cu_expected = matches!(
            info.program_id.as_array(),
            &SYSTEM_PROGRAM_ID | &COMPUTE_BUDGET_ID
        );
        let valid_consumed = no_cu_expected || info.units_consumed.is_some();
        let valid_allowance = no_cu_expected || info.consumption_allowance.is_some();
        ensure!(valid_consumed, "Missing units consumed");
        ensure!(valid_allowance, "Missing consumption allowance");

        self.infos.push(info);

        Ok(())
    }

    pub fn build_compute_infos(self) -> ParseResult<Vec<GroupedParsedLogs>> {
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
            .sorted_by_key(|info| info.invocation_index)
            .partition(|info| info.parent_index.is_none());

        let mut parent_instructions = parent_infos
            .into_iter()
            .map(|info| GroupedParsedLogs {
                parent: info,
                children: vec![],
            })
            .collect_vec();

        ensure!(
            parents.len() == parent_instructions.len(),
            "Parent length mismatch"
        );

        for child in children_infos.into_iter() {
            let parent_idx = child
                .parent_index
                .ok_or(anyhow::Error::msg("Child should have parent index"))?;

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
    compute_map: Vec<GroupedParsedLogs>,
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
