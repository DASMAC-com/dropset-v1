use dropset_interface::{
    pack::unpack_amount_and_optional_sector_index,
    state::{market_seat::MarketSeat, node::Node},
};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, ProgramResult};

use crate::{
    context::deposit_withdraw_context::DepositWithdrawContext,
    shared::{
        market_operations::{find_mut_seat_with_hint, insert_market_seat},
        token_utils::market_transfers::deposit_non_zero_to_market,
    },
};

/// User deposits tokens and updates or registers their seat.
///
/// 1) User provided a sector index hint: update an existing seat.
///   - Try to find the seat with the user's sector index hint.
///   - If invalid return early, otherwise update the seat with the amount deposited.
///
/// 2) The user didn't provide a sector index hint: register a new seat.
///   - Check if the account needs extra storage and resize it if so.
///   - Then register the user's new seat at the proper index with the amount deposited data.
///   - If the user already exists, return an error instead of inserting.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in [`dropset_interface::instructions::deposit::Deposit`]
pub unsafe fn process_deposit(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let (amount, hint) = unpack_amount_and_optional_sector_index(instruction_data)?;

    let mut ctx = unsafe { DepositWithdrawContext::load(accounts) }?;
    let amount_deposited = unsafe { deposit_non_zero_to_market(&ctx, amount) }?;

    // 1) Update an existing seat.
    if let Some(index) = hint {
        // Safety: Scoped mutable borrow of the market account to mutate the user's seat.
        let market = unsafe { ctx.market_account.load_unchecked_mut() };
        Node::check_in_bounds(market.sectors, index)?;
        // Safety: The index hint was just verified as in-bounds.
        let seat = unsafe { find_mut_seat_with_hint(market, index, ctx.user.key()) }?;

        if ctx.mint.is_base_mint {
            seat.set_base_available(
                seat.base_available()
                    .checked_add(amount_deposited)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
            seat.set_base_deposited(
                seat.base_deposited()
                    .checked_add(amount_deposited)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
        } else {
            seat.set_quote_available(
                seat.quote_available()
                    .checked_add(amount_deposited)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
            seat.set_quote_deposited(
                seat.quote_deposited()
                    .checked_add(amount_deposited)
                    .ok_or(ProgramError::ArithmeticOverflow)?,
            );
        }
    } else {
        // 2) Register a new seat.
        // Safety: Scoped immutable borrow of the market account, checks the number of free sectors.
        let needs_resize = unsafe { ctx.market_account.load_unchecked() }
            .header
            .num_free_sectors()
            == 0;

        if needs_resize {
            // Safety: Scoped mutable borrow to resize the market account and add a new sector/node.
            unsafe { ctx.market_account.resize(ctx.user, 1) }?;
        }

        // Safety: Scoped mutable borrow of market account data to insert the new seat.
        let mut market = unsafe { ctx.market_account.load_unchecked_mut() };

        let seat = if ctx.mint.is_base_mint {
            MarketSeat::new(*ctx.user.key(), amount_deposited, 0)
        } else {
            MarketSeat::new(*ctx.user.key(), 0, amount_deposited)
        };

        // Attempts to insert the user into the linked list. If the user already exists, this fails.
        insert_market_seat(&mut market.seat_list(), seat)?;
    }

    Ok(())
}
