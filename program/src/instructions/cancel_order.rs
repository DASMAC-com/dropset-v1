//! See [`process_cancel_order`].

use dropset_interface::{
    events::CancelOrderEventInstructionData,
    instructions::CancelOrderInstructionData,
    state::{
        node::Node,
        sector::SectorIndex,
    },
};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::{
    context::{
        place_order_context::PlaceOrderContext,
        EventBufferContext,
    },
    events::EventBuffer,
    shared::{
        order_operations::load_mut_order_from_sector_index,
        seat_operations::find_mut_seat_with_hint,
    },
};

/// Instruction handler logic for cancelling a user's bid or ask order on the market's order book.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in
/// [`dropset_interface::instructions::generated_pinocchio::CancelOrder`].
#[inline(never)]
pub unsafe fn process_cancel_order<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let CancelOrderInstructionData {
        encoded_price,
        is_bid,
        user_sector_index_hint,
    } = CancelOrderInstructionData::unpack_pinocchio(instruction_data)?;
    let mut ctx = PlaceOrderContext::load(accounts)?;

    // Update the user's market seat balances and remove the order sector index from the user order
    // sector indices.
    // Then return the order sector index so it can be used to remove the order from the orders
    // collection.
    let order_sector_index = {
        // Safety: Scoped mutable borrow of the market account.
        let market = unsafe { ctx.market_account.load_unchecked_mut() };
        let user_seat = find_mut_seat_with_hint(market, user_sector_index_hint, ctx.user.key())?;
        // 1. Update the user's seat to no longer include the price to order sector index mapping.
        // 2. Update the user's collateral in their market seat by returning the matching amount
        //    from the order.
        if is_bid {
            // 1. Remove the seat from the user's placed bids.
            let order_sector_index = SectorIndex::from_le_bytes(
                user_seat.user_order_sectors.bids.remove(encoded_price)?,
            );

            // Safety: The order sector index returned from the `remove` method still points to a
            // sector with a valid order.
            let order = unsafe { load_mut_order_from_sector_index(market, order_sector_index) };

            // 2. If the user placed a bid, they provided quote as collateral. Return it to their
            //    seat balance.
            user_seat.try_increment_quote_available(order.quote_remaining())?;

            order_sector_index
        } else {
            // 1. Remove the seat from the user's placed asks.
            let order_sector_index = SectorIndex::from_le_bytes(
                user_seat.user_order_sectors.asks.remove(encoded_price)?,
            );

            // Safety: The order sector index returned from the `remove` method still points to a
            // sector with a valid order.
            let order = unsafe { load_mut_order_from_sector_index(market, order_sector_index) };

            // 2. If the user placed an ask, they provided base as collateral. Return it to their
            //    seat balance.
            user_seat.try_increment_base_available(order.base_remaining())?;

            order_sector_index
        }
    };

    {
        // Safety: Scoped mutable borrow of the market account to remove the order from the orders
        // collection.
        let mut market = unsafe { ctx.market_account.load_unchecked_mut() };
        Node::check_in_bounds(market.sectors, order_sector_index)?;
        // Find and remove the order given the order sector index.
        // Safety: The index was just verified as in-bounds and is still pointing to a valid order.
        market.order_list().remove_at(order_sector_index);
    }

    event_buffer.add_to_buffer(
        CancelOrderEventInstructionData::new(is_bid, user_sector_index_hint),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
