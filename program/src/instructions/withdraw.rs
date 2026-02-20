//! See [`process_withdraw`].

use dropset_interface::{
    events::WithdrawEventInstructionData,
    instructions::WithdrawInstructionData,
    state::sector::Sector,
};
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::{
    context::{
        deposit_withdraw_context::DepositWithdrawContext,
        EventBufferContext,
    },
    events::EventBuffer,
    shared::{
        seat_operations::load_mut_seat_with_hint,
        token_utils::market_transfers::withdraw_non_zero_from_market,
    },
};

/// Instruction handler logic for withdrawing funds from a market seat.
///
/// # Safety
///
/// Caller upholds the safety contract detailed in
/// [`dropset_interface::instructions::generated_program::Withdraw`].
#[inline(never)]
pub unsafe fn process_withdraw<'a>(
    accounts: &'a [AccountView],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let WithdrawInstructionData {
        amount,
        sector_index_hint,
    } = WithdrawInstructionData::unpack_untagged(instruction_data)?;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { DepositWithdrawContext::load(accounts) }?;
    unsafe {
        withdraw_non_zero_from_market(
            &ctx.user_ata,
            &ctx.market_ata,
            &ctx.market_account,
            &ctx.mint,
            amount,
        )
    }?;

    // Safety: Scoped mutable borrow of market account data to update the user's seat.
    let mut market = unsafe { ctx.market_account.load_unchecked_mut() };

    // Find the seat with the index hint or fail and return early.
    Sector::check_in_bounds(market.sectors, sector_index_hint)?;
    // Safety: The hint was just verified as in-bounds.
    let seat =
        unsafe { load_mut_seat_with_hint(&mut market, sector_index_hint, ctx.user.address()) }?;

    // Update the market seat available/deposited, checking for underflow, as that means the user
    // tried to withdraw more than they have available.
    if ctx.mint.is_base_mint {
        seat.try_decrement_base_available(amount)?;
    } else {
        seat.try_decrement_quote_available(amount)?;
    }

    event_buffer.add_to_buffer(
        WithdrawEventInstructionData::new(amount, ctx.mint.is_base_mint),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
