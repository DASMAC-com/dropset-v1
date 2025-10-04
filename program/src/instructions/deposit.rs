use dropset_interface::{instructions::amount::AmountInstructionData, state::transmutable::load};
use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::context::deposit_withdraw_context::DepositWithdrawContext;

pub fn process_deposit(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let ctx = DepositWithdrawContext::load(accounts)?;

    // Safety: All bit patterns are valid.
    let amount = unsafe { load::<AmountInstructionData>(instruction_data) }?.amount();

    Ok(())
}
