use dropset_interface::{
    error::DropsetError,
    instructions::generated_pinocchio::WithdrawInstructionData,
    state::node::Node,
};
use pinocchio::{
    account_info::AccountInfo,
    ProgramResult,
};

use crate::{
    context::deposit_withdraw_context::DepositWithdrawContext,
    shared::{
        market_operations::find_mut_seat_with_hint,
        token_utils::market_transfers::withdraw_non_zero_from_market,
    },
};

/// User withdraws tokens and updates their seat.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in
/// [`dropset_interface::instructions::withdraw::Withdraw`]
pub unsafe fn process_withdraw(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let WithdrawInstructionData {
        amount,
        sector_index_hint,
    } = WithdrawInstructionData::unpack(instruction_data)?;

    // Safety: Scoped immutable borrow of market, user token, and market token accounts to validate.
    let mut ctx = unsafe { DepositWithdrawContext::load(accounts) }?;
    unsafe { withdraw_non_zero_from_market(&ctx, amount) }?;

    // Safety: Scoped mutable borrow of market account data to update the user's seat.
    let market = unsafe { ctx.market_account.load_unchecked_mut() };

    // Find the seat with the index hint or fail and return early.
    Node::check_in_bounds(market.sectors, sector_index_hint)?;
    // Safety: The hint was just verified as in-bounds.
    let seat = unsafe { find_mut_seat_with_hint(market, sector_index_hint, ctx.user.key()) }?;

    // Update the market seat available/deposited, checking for underflow, as that means the user
    // tried to withdraw more than they have available.
    if ctx.mint.is_base_mint {
        seat.set_base_available(
            seat.base_available()
                .checked_sub(amount)
                .ok_or(DropsetError::InsufficientUserBalance)?,
        );
        seat.set_base_deposited(
            seat.base_deposited()
                .checked_sub(amount)
                .ok_or(DropsetError::InsufficientUserBalance)?,
        );
    } else {
        seat.set_quote_available(
            seat.quote_available()
                .checked_sub(amount)
                .ok_or(DropsetError::InsufficientUserBalance)?,
        );
        seat.set_quote_deposited(
            seat.quote_deposited()
                .checked_sub(amount)
                .ok_or(DropsetError::InsufficientUserBalance)?,
        );
    }

    Ok(())
}
