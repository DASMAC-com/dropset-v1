use std::fmt::{
    self,
    Debug,
    Display,
    Formatter,
};

use colored::Colorize;
use dropset_interface::{
    instructions::DropsetInstruction,
    state::SYSTEM_PROGRAM_ID,
};
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction::SystemInstruction;
use spl_associated_token_account_interface::instruction::AssociatedTokenAccountInstruction;
use spl_token_2022_interface::instruction::TokenInstruction as Token2022Instruction;
use spl_token_interface::instruction::TokenInstruction;

use crate::{
    logs::LogColor,
    transaction_parser::{
        ParsedInstruction,
        ParsedTransaction,
    },
    SPL_ASSOCIATED_TOKEN_ACCOUNT_ID,
    SPL_TOKEN_2022_ID,
    SPL_TOKEN_ID,
};

pub struct PrettyTransaction<'a> {
    /// The amount of spaces preceding each line in the output.
    pub indent: u8,
    /// The parsed transaction.
    pub transaction: &'a ParsedTransaction,
}

pub struct PrettyInstruction<'a> {
    pub instruction: &'a ParsedInstruction,
    pub outer: bool,
}

impl Display for PrettyInstruction<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = format_instruction_info(self.instruction, self.outer);
        f.write_str(&s)
    }
}

impl Display for PrettyTransaction<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for outer in &self.transaction.instructions {
            let outer_header = format_instruction_info(&outer.outer_instruction, true);
            let indentation = " ".repeat(self.indent as usize);

            writeln!(f, "{indentation}{outer_header}")?;

            for (i, inner) in outer.inner_instructions.iter().enumerate() {
                let pretty_instruction = PrettyInstruction {
                    instruction: inner,
                    outer: false,
                };
                let text = format!("    {:>2}. {}", i + 1, pretty_instruction);
                let colored = text.color(LogColor::FadedGray);
                writeln!(f, "{indentation}{colored}")?;
            }
        }

        Ok(())
    }
}

#[derive(strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
enum KnownProgram {
    Dropset,
    SplToken,
    SplToken2022,
    SystemProgram,
    AssociatedTokenAccount,
}

impl KnownProgram {
    pub const fn from_program_id(program_id: &Pubkey) -> Option<Self> {
        match program_id.to_bytes() {
            dropset::ID => Some(Self::Dropset),
            SPL_TOKEN_ID => Some(Self::SplToken),
            SPL_TOKEN_2022_ID => Some(Self::SplToken2022),
            SYSTEM_PROGRAM_ID => Some(Self::SystemProgram),
            SPL_ASSOCIATED_TOKEN_ACCOUNT_ID => Some(Self::AssociatedTokenAccount),
            _ => None,
        }
    }

    pub fn instruction_name(&self, instruction_data: &[u8]) -> String {
        match self {
            Self::Dropset => {
                let tag = instruction_data
                    .first()
                    .expect("Dropset instruction should have at least one byte");
                let dropset_tag = DropsetInstruction::try_from_u8(*tag, || anyhow::Error::msg(""))
                    .expect("Dropset instruction tag should be valid");
                enum_name(&dropset_tag)
            }
            Self::SplToken => {
                let token_instruction = TokenInstruction::unpack(instruction_data)
                    .expect("Should unpack token instruction data");
                enum_name(&token_instruction)
            }
            Self::SplToken2022 => {
                let token_instruction = Token2022Instruction::unpack(instruction_data)
                    .expect("Should unpack token 2022 instruction data");
                enum_name(&token_instruction)
            }
            Self::SystemProgram => {
                let system_instruction =
                    bincode::deserialize::<SystemInstruction>(instruction_data)
                        .expect("Should unpack system instruction data");
                enum_name(&system_instruction)
            }
            Self::AssociatedTokenAccount => {
                let spl_ata_instruction =
                    borsh::from_slice::<AssociatedTokenAccountInstruction>(instruction_data)
                        .expect("Should unpack spl ata instruction data");
                enum_name(&spl_ata_instruction)
            }
        }
    }
}

fn format_instruction_info(instruction: &ParsedInstruction, outer: bool) -> String {
    let program_id = &instruction.program_id;
    let known_program = KnownProgram::from_program_id(program_id);

    let first_acc = instruction
        .accounts
        .iter()
        .find_map(|acc| acc.signer.then_some(acc.pubkey))
        .map(|acc| format!("sender: {acc}"))
        .unwrap_or_default();

    match known_program {
        Some(program) => {
            let (name, first_acc) = match outer {
                true => (program.to_string().color(LogColor::Debug), Some(first_acc)),
                false => (program.to_string().bright_black(), None),
            };
            format!(
                "{name}::{}{}",
                program.instruction_name(&instruction.data),
                first_acc.unwrap_or_default()
            )
        }
        None => format!("Unknown program: {:?}", program_id)
            .color(LogColor::Warning)
            .to_string(),
    }
}

// This should only be used with enums. It assumes that `Debug` will print the value like `Ident {`.
fn enum_name<T: Debug>(value: &T) -> String {
    let s = format!("{:?}", value);
    s.split_once([' ', '{'])
        .map(|(n, _)| n)
        .unwrap_or(&s)
        .into()
}
