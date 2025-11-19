//! See [`process_deposit`].

use dropset_interface::{
    events::DepositEventInstructionData,
    instructions::generated_pinocchio::*,
    state::{
        market_seat::MarketSeat,
        node::Node,
        sector::NIL,
    },
};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::{
    context::{
        deposit_withdraw_context::DepositWithdrawContext,
        EventBufferContext,
    },
    events::EventBuffer,
    shared::{
        market_operations::{
            find_mut_seat_with_hint,
            insert_market_seat,
        },
        token_utils::market_transfers::deposit_non_zero_to_market,
    },
};

/// Instruction handler logic for depositing funds into a market seat.
///
/// There are two paths:
///
/// 1) User provided a non-NIL sector index hint: update an existing seat.
///   - Try to find the seat with the user's sector index hint.
///   - If invalid return early, otherwise update the seat with the amount deposited.
///
/// 2) The user didn't provide a non-NIL sector index hint: register a new seat.
///   - Check if the account needs extra storage and resize it if so.
///   - Then register the user's new seat at the proper index with the amount deposited data.
///   - If the user already exists, return an error instead of inserting.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in [`Deposit`].
#[inline(never)]
pub unsafe fn process_deposit<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let DepositInstructionData {
        amount,
        sector_index_hint,
    } = DepositInstructionData::unpack(instruction_data)?;

    let mut ctx = unsafe { DepositWithdrawContext::load(accounts) }?;
    let amount_deposited = unsafe { deposit_non_zero_to_market(&ctx, amount) }?;

    // 1) Update an existing seat.
    let sector_index = if sector_index_hint != NIL {
        // Safety: Scoped mutable borrow of the market account to mutate the user's seat.
        let market = unsafe { ctx.market_account.load_unchecked_mut() };
        Node::check_in_bounds(market.sectors, sector_index_hint)?;
        // Safety: The index hint was just verified as in-bounds.
        let seat = unsafe { find_mut_seat_with_hint(market, sector_index_hint, ctx.user.key()) }?;

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

        sector_index_hint
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
        insert_market_seat(&mut market.seat_list(), seat)?
    };

    event_buffer.add_to_buffer(
        DepositEventInstructionData::new(amount_deposited, ctx.mint.is_base_mint, sector_index),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
