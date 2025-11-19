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

use crate::instructions::*;

program_entrypoint!(process_instruction);
no_allocator!();
nostd_panic_handler!();

// `inline(never)` because the event buffer + batch instruction data causes the program to exceed
// the 4096 stack frame size very quickly.
#[inline(never)]
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let [tag, remaining @ ..] = instruction_data else {
        return Err(DropsetError::InvalidInstructionTag.into());
    };

    // Safety: No account data is currently borrowed. CPIs to this program must ensure they do not
    // hold references to the account data used in each instruction.
    unsafe {
        match DropsetInstruction::try_from(*tag)? {
            DropsetInstruction::RegisterMarket => process_register_market(accounts, remaining),
            DropsetInstruction::Deposit => process_deposit(accounts, remaining),
            DropsetInstruction::Withdraw => process_withdraw(accounts, remaining),
            DropsetInstruction::CloseSeat => process_close_seat(accounts, remaining),
            DropsetInstruction::FlushEvents => process_flush_events(accounts, remaining),
            DropsetInstruction::Batch => process_batch(accounts, remaining),
        }
    }
}
