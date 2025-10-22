use dropset_interface::{
    error::DropsetError,
    instructions::InstructionTag,
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

#[inline(always)]
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
        match InstructionTag::try_from(*tag)? {
            InstructionTag::RegisterMarket => process_register_market(accounts, remaining),
            InstructionTag::Deposit => process_deposit(accounts, remaining),
            InstructionTag::Withdraw => process_withdraw(accounts, remaining),
            InstructionTag::CloseSeat => process_close_seat(accounts, remaining),
            InstructionTag::FlushEvents => process_flush_events(accounts, remaining),
        }
    }
}
