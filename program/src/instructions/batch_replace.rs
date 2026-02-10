//! See [`process_batch_replace`].

use pinocchio::{
    account::AccountView,
    ProgramResult,
};

/// Handler logic for executing multiple instructions in a single atomic batch.
///
/// # Safety
///
/// Since the accounts borrowed depend on the inner batch instructions, the most straightforward
/// safety contract is simply ensuring that **no Solana account data is currently borrowed** prior
/// to calling this instruction.
#[inline(never)]
pub fn process_batch_replace(_accounts: &[AccountView], _instruction_data: &[u8]) -> ProgramResult {
    Ok(())
}
