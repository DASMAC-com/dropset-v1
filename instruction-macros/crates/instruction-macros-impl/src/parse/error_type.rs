//! See [`ErrorType`].

use crate::parse::error_path::ErrorPath;

/// Maps high-level instruction validation errors to concrete `ProgramError` variants
/// for each supported feature/target.
pub enum ErrorType {
    IncorrectNumAccounts,
    InvalidInstructionData,
}

impl ErrorType {
    pub fn to_path(&self) -> ErrorPath {
        let base = "::solana_program_error::ProgramError";
        match self {
            ErrorType::InvalidInstructionData => ErrorPath::new(base, "InvalidInstructionData"),
            ErrorType::IncorrectNumAccounts => ErrorPath::new(base, "NotEnoughAccountKeys"),
        }
    }
}
