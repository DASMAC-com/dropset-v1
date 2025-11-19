//! Solana program entrypoint.
//!
//! Forwards incoming instructions from the runtime into the program’s core instruction processing
//! logic.

use dropset_interface::{
    error::DropsetError,
    instructions::DropsetInstruction,
};
use pinocchio::{
    account_info::AccountInfo,
    no_allocator,
    nostd_panic_handler,
    program_entrypoint,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::{
    context::EventBufferContext,
    events::EventBuffer,
    instructions::*,
};

program_entrypoint!(process_instruction);
no_allocator!();
nostd_panic_handler!();

// `inline(never)` because the event buffer + batch instruction data causes the program to exceed
// the 4096 stack frame size very quickly.
#[inline(never)]
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data_with_tag: &[u8],
) -> ProgramResult {
    let [tag, instruction_data @ ..] = instruction_data_with_tag else {
        return Err(DropsetError::InvalidInstructionTag.into());
    };

    let instruction_tag = DropsetInstruction::try_from(*tag)?;

    let event_buffer = &mut EventBuffer::new(instruction_tag);

    // Safety: No account data is currently borrowed. CPIs to this program must ensure they do not
    // hold references to the account data used in each instruction.
    let event_buffer_context = unsafe {
        match instruction_tag {
            DropsetInstruction::RegisterMarket => {
                process_register_market(accounts, instruction_data, event_buffer)
            }
            DropsetInstruction::Deposit => {
                process_deposit(accounts, instruction_data, event_buffer)
            }
            DropsetInstruction::Withdraw => {
                process_withdraw(accounts, instruction_data, event_buffer)
            }
            DropsetInstruction::CloseSeat => {
                process_close_seat(accounts, instruction_data, event_buffer)
            }
            DropsetInstruction::FlushEvents => {
                return process_flush_events(accounts, instruction_data)
            }
            DropsetInstruction::Batch => return process_batch(accounts, instruction_data),
        }
    }?;

    let EventBufferContext {
        event_authority,
        market_account,
    } = event_buffer_context;

    // Safety: The `market_account` is not currently borrowed in any capacity.
    unsafe { event_buffer.flush_events(event_authority, market_account) }
}
