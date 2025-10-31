use crate::{
    parse::error_path::ErrorPath,
    render::Feature,
};

pub enum ErrorType {
    IncorrectNumAccounts,
    InvalidInstructionData,
}

impl ErrorType {
    pub fn to_path(&self, feature: Feature) -> ErrorPath {
        let base = match feature {
            Feature::Client => "::solana_sdk::program_error::ProgramError",
            Feature::Pinocchio => "::pinocchio::program_error::ProgramError",
            Feature::SolanaProgram => "::solana_sdk::program_error::ProgramError",
        };
        match self {
            ErrorType::InvalidInstructionData => ErrorPath::new(base, "InvalidInstructionData"),
            ErrorType::IncorrectNumAccounts => ErrorPath::new(base, "NotEnoughAccountKeys"),
        }
    }
}
