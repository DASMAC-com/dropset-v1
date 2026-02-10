//! See [`process_cancel_order`].

#[cfg(feature = "debug")]
use dropset_interface::events::CancelOrderEventInstructionData;
use dropset_interface::{
    instructions::CancelOrderInstructionData,
    state::sector::{
        Sector,
        SectorIndex,
    },
};
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::{
    context::{
        mutate_orders_context::MutateOrdersContext,
        EventBufferContext,
    },
    events::EventBuffer,
    shared::{
        order_operations::load_order_from_sector_index,
        seat_operations::{
            load_mut_seat_with_hint,
            load_mut_seat_with_hint_unchecked,
        },
    },
};

/// Instruction handler logic for cancelling a user's bid or ask order on the market's order book.
///
/// # Safety
///
/// Caller upholds the safety contract detailed in
/// [`dropset_interface::instructions::generated_program::CancelOrder`].
#[inline(never)]
pub unsafe fn process_cancel_order<'a>(
    accounts: &'a [AccountView],
    instruction_data: &[u8],
    _event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let CancelOrderInstructionData {
        encoded_price,
        is_bid,
        user_sector_index_hint,
    } = CancelOrderInstructionData::unpack_untagged(instruction_data)?;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts) }?;

    // Safety: The market account is currently not borrowed in any capacity.
    let mut market = unsafe { ctx.market_account.load_unchecked_mut() };

    // Remove the order from the user seat's order sectors mapping.
    let order_sector_index = {
        Sector::check_in_bounds(market.sectors, user_sector_index_hint)?;
        // Safety: The user sector index hint was just verified in-bounds.
        let user_seat = unsafe {
            load_mut_seat_with_hint(&mut market, user_sector_index_hint, ctx.user.address())
        }?;
        if is_bid {
            SectorIndex::from_le_bytes(
                user_seat
                    .user_order_sectors
                    .bids
                    .find_remove(encoded_price)?,
            )
        } else {
            SectorIndex::from_le_bytes(
                user_seat
                    .user_order_sectors
                    .asks
                    .find_remove(encoded_price)?,
            )
        }
    };

    // The safety comment below explains why this isn't explicitly necessary.
    debug_assert!(Sector::check_in_bounds(market.sectors, order_sector_index).is_ok());

    // Load the order given the order sector index.
    let order = {
        // Safety: The order sector index returned from the `remove` method still points to a
        // sector with a valid order. All order sector indices in a user seat are thus in-bounds and
        // don't need to be explicitly verified as in-bounds.
        load_order_from_sector_index(&market, order_sector_index)
    };

    // Increment the user's collateral in their market seat by the amount remaining in the order.
    if is_bid {
        // If the user placed a bid, they provided quote as collateral.
        let order_size_remaining = order.quote_remaining();
        // Safety: The seat hint was already validated as in-bounds. It could only possibly be out
        // of bounds now if the account data size was just reduced, which it was not.
        let user_seat = load_mut_seat_with_hint_unchecked(&mut market, user_sector_index_hint);
        user_seat.try_increment_quote_available(order_size_remaining)?;
    } else {
        // If the user placed an ask, they provided base as collateral.
        let order_size_remaining = order.base_remaining();
        // Safety: The seat hint was already validated as in-bounds. It could only possibly be out
        // of bounds now if the account data size was just reduced, which it was not.
        let user_seat = load_mut_seat_with_hint_unchecked(&mut market, user_sector_index_hint);
        user_seat.try_increment_base_available(order_size_remaining)?;
    }

    // Remove the order at the order sector index from the appropriate orders collection.
    unsafe {
        // Safety: The order sector index from the `remove` method is still in-bounds.
        if is_bid {
            market.bids().remove_at(order_sector_index);
        } else {
            market.asks().remove_at(order_sector_index);
        }
    }

    #[cfg(feature = "debug")]
    _event_buffer.add_to_buffer(
        CancelOrderEventInstructionData::new(is_bid, user_sector_index_hint),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
