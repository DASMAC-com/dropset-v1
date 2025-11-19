//! See [`process_flush_events`].

use pinocchio::{
    account_info::AccountInfo,
    ProgramResult,
};

use crate::context::flush_events_context::FlushEventsContext;

/// Handler logic for flushing/consuming pending intra-transaction events.
///
/// On-chain, this instruction only verifies the event-authority account. Parsing events in the
/// instruction data is performed off-chain, so the instruction data is unused here.
#[inline(never)]
pub fn process_flush_events(accounts: &[AccountInfo], _instruction_data: &[u8]) -> ProgramResult {
    FlushEventsContext::load(accounts)?;

    Ok(())
}
