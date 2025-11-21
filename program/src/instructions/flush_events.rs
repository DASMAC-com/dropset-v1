//! See [`process_flush_events`].

use pinocchio::{
    account_info::AccountInfo,
    ProgramResult,
};

/// Handler logic for flushing/consuming pending intra-transaction events associated with a market.
#[inline(never)]
pub fn process_flush_events(_accounts: &[AccountInfo], _instruction_data: &[u8]) -> ProgramResult {
    Ok(())
}
