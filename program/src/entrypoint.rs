use crate::instructions::*;
use dropset_interface::{error::DropsetError, instructions::InstructionTag};
use pinocchio::{account_info::AccountInfo, pubkey::Pubkey, ProgramResult};

#[inline(always)]
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let [tag, remaining @ ..] = instruction_data else {
        return Err(DropsetError::InvalidInstructionTag.into());
    };

    match InstructionTag::try_from(*tag)? {
        InstructionTag::RegisterMarket => process_register_market(accounts, remaining),
        InstructionTag::Deposit => process_deposit(accounts, remaining),
        InstructionTag::Withdraw => process_withdraw(accounts, remaining),
        InstructionTag::Close => process_close(accounts, remaining),
        InstructionTag::FlushEvents => process_flush_events(accounts, remaining),
    }
}
