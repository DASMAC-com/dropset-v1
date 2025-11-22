//! Pretty-prints parsed transactions with filtered, indexed, and indented instruction traces.

use std::{
    collections::HashSet,
    fmt::{
        self,
        Display,
        Formatter,
    },
};

use colored::Colorize;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
};
use transaction_parser::client_rpc::ParsedTransaction;

use crate::{
    fmt_kv,
    logs::LogColor,
    pretty::instruction::PrettyInstruction,
};

pub struct PrettyTransaction<'a> {
    /// The transaction signature.
    pub signature: Signature,
    /// The sender of the transaction.
    pub sender: Pubkey,
    /// The amount of spaces preceding each line in the output.
    pub indent_size: usize,
    /// The parsed transaction.
    pub transaction: &'a ParsedTransaction,
    /// Instruction program ID filter; i.e., only prints instructions with these IDs.
    pub instruction_filter: &'a HashSet<Pubkey>,
}

impl Display for PrettyTransaction<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let filtered = self
            .transaction
            .instructions
            .iter()
            .filter(|instruction| {
                self.instruction_filter
                    .contains(&instruction.outer_instruction.program_id)
            })
            .collect::<Vec<_>>();

        if !filtered.is_empty() {
            let signature = fmt_kv!("Signature", self.signature, LogColor::Header);
            let sender = fmt_kv!("Sender", self.sender, LogColor::Gray);

            writeln!(f, "{signature}")?;
            writeln!(f, "{sender}")?;
        }

        let mut i: usize = 0;
        for outer in filtered {
            i += 1;
            let indentation = " ".repeat(self.indent_size);
            let pretty_outer = PrettyInstruction {
                instruction: &outer.outer_instruction,
                outer: true,
            };

            let idx = format_instruction_index(i);
            writeln!(f, "{idx}{indentation}{}", pretty_outer)?;

            for inner in outer.inner_instructions.iter() {
                i += 1;

                let pretty_instruction = PrettyInstruction {
                    instruction: inner,
                    outer: false,
                };
                let inner_indent = indentation.repeat(
                    inner
                        .compute_info
                        .as_ref()
                        .map(|cu| cu.stack_height)
                        .unwrap_or(1),
                );
                let text = format!("{inner_indent} {}", pretty_instruction);
                let colored = text.color(LogColor::FadedGray);
                let idx = format_instruction_index(i);
                writeln!(f, "{idx} {colored}")?;
            }
        }

        Ok(())
    }
}

fn format_instruction_index(idx: usize) -> String {
    format!("{idx:>2}").color(LogColor::FadedGray).to_string()
}
