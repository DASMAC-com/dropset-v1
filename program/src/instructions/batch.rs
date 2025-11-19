//! See [`process_batch`].

use pinocchio::{
    account_info::AccountInfo,
    ProgramResult,
};

/// Handler logic for executing multiple instructions in a single atomic batch.
#[inline(never)]
pub fn process_batch(_accounts: &[AccountInfo], _instruction_data: &[u8]) -> ProgramResult {
    Ok(())
}
