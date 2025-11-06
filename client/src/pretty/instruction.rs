use std::fmt::{
    self,
    Debug,
    Display,
    Formatter,
};

use colored::{
    Color,
    Colorize,
};
use dropset_interface::{
    instructions::DropsetInstruction,
    state::SYSTEM_PROGRAM_ID,
};
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_system_interface::instruction::SystemInstruction;
use spl_associated_token_account_interface::instruction::AssociatedTokenAccountInstruction;
use spl_token_2022_interface::instruction::TokenInstruction as Token2022Instruction;
use spl_token_interface::instruction::TokenInstruction;

use crate::{
    logs::LogColor,
    transaction_parser::ParsedInstruction,
    COMPUTE_BUDGET_ID,
    SPL_ASSOCIATED_TOKEN_ACCOUNT_ID,
    SPL_TOKEN_2022_ID,
    SPL_TOKEN_ID,
};

pub struct PrettyInstruction<'a> {
    pub instruction: &'a ParsedInstruction,
    pub outer: bool,
}

impl Display for PrettyInstruction<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let instruction = self.instruction;
        let program_id = &instruction.program_id;
        let known_program = KnownProgram::from_program_id(program_id);

        let name_highlight_color = match (self.outer, known_program.is_some()) {
            (true, true) => LogColor::Debug.into(),
            (false, true) => Color::BrightBlack,
            (_, false) => LogColor::Warning.into(),
        };

        let compute_units = self.format_cu();

        let (program_name, instruction_name) = match known_program {
            Some(known) => (known.to_string(), known.instruction_name(&instruction.data)),
            None => (program_id.to_string(), "UnknownInstruction".into()),
        };

        let colored_name = program_name.color(name_highlight_color);
        let s = format!("{colored_name}::{instruction_name}{compute_units}");

        f.write_str(&s)
    }
}

impl PrettyInstruction<'_> {
    fn format_cu(&self) -> String {
        self.instruction
            .compute_info
            .as_ref()
            .and_then(|cu| cu.units_consumed)
            .map(|cu| {
                let cu_highlight_color = match self.outer {
                    true => color_from_value(cu),
                    false => Color::BrightBlack,
                };
                let highlighted_cu = format!("{cu}").color(cu_highlight_color);
                match self.outer {
                    true => format!(" consumed {} compute units", highlighted_cu)
                        .color(LogColor::FadedGray)
                        .to_string(),
                    false => format!(" — {} cu", highlighted_cu),
                }
            })
            .unwrap_or_default()
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
    ComputeBudget,
}

impl KnownProgram {
    pub const fn from_program_id(program_id: &Pubkey) -> Option<Self> {
        match program_id.to_bytes() {
            dropset::ID => Some(Self::Dropset),
            SPL_TOKEN_ID => Some(Self::SplToken),
            SPL_TOKEN_2022_ID => Some(Self::SplToken2022),
            SYSTEM_PROGRAM_ID => Some(Self::SystemProgram),
            SPL_ASSOCIATED_TOKEN_ACCOUNT_ID => Some(Self::AssociatedTokenAccount),
            COMPUTE_BUDGET_ID => Some(Self::ComputeBudget),
            _ => None,
        }
    }

    pub fn instruction_name(&self, instruction_data: &[u8]) -> String {
        match self {
            Self::Dropset => {
                let tag = instruction_data
                    .first()
                    .expect("Dropset instruction should have at least one byte");
                let dropset_tag = DropsetInstruction::try_from(*tag)
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
                        .expect("Should deserialize system instruction data");
                enum_name(&system_instruction)
            }
            Self::AssociatedTokenAccount => {
                let spl_ata_instruction =
                    borsh::from_slice::<AssociatedTokenAccountInstruction>(instruction_data)
                        .expect("Should deserialize spl ata instruction data");
                enum_name(&spl_ata_instruction)
            }
            Self::ComputeBudget => {
                let compute_budget_instruction =
                    bincode::deserialize::<ComputeBudgetInstruction>(instruction_data)
                        .expect("Should deserialize compute budget instruction data");
                enum_name(&compute_budget_instruction)
            }
        }
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

const MAX_CU_SATURATION: u64 = 50000;

// Increase color saturation as the CU goes from 0 -> MAX_CU_SATURATION.
fn color_from_value(v: u64) -> Color {
    let t = (v.min(MAX_CU_SATURATION) as f64 / 50000.0).powf(1.3);
    let lerp = |a: f64, b: f64| (a + (b - a) * t).round() as u8;
    Color::TrueColor {
        r: lerp(150.0, 255.0),
        g: lerp(120.0, 160.0),
        b: lerp(100.0, 20.0),
    }
}
