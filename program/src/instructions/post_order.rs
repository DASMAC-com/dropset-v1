//! See [`process_post_order`].

#[cfg(feature = "debug")]
use dropset_interface::events::PostOrderEventInstructionData;
use dropset_interface::{
    error::DropsetError,
    instructions::PostOrderInstructionData,
    state::{
        asks_dll::AskOrders,
        bids_dll::BidOrders,
        market::MarketRefMut,
        order::{
            Order,
            OrdersCollection,
        },
        sector::{
            Sector,
            SectorIndex,
        },
    },
};
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};
use price::to_order_info;

use crate::{
    context::{
        mutate_orders_context::MutateOrdersContext,
        EventBufferContext,
    },
    events::EventBuffer,
    shared::{
        order_operations::insert_order,
        seat_operations::load_mut_seat_with_hint,
    },
};

/// Instruction handler logic for posting a user's bid or ask order on the market's order book.
///
/// # Safety
///
/// Caller upholds the safety contract detailed in
/// [`dropset_interface::instructions::generated_program::PostOrder`].
#[inline(never)]
pub unsafe fn process_post_order<'a>(
    accounts: &'a [AccountView],
    instruction_data: &[u8],
    _event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let PostOrderInstructionData {
        order_info_args,
        is_bid,
        user_sector_index_hint,
    } = PostOrderInstructionData::unpack_untagged(instruction_data)?;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts) }?;

    let order_info = to_order_info(order_info_args).map_err(DropsetError::from)?;

    let (base_atoms, quote_atoms) = (order_info.base_atoms, order_info.quote_atoms);

    // To avoid convoluted borrow checking rules, optimistically insert the order with the index
    // hint passed in, assuming it's valid. It's verified later when mutating the market seat.
    let order = Order::new(order_info, user_sector_index_hint);
    let le_encoded_price = *order.le_encoded_price();

    // Safety: The market account is currently not borrowed in any capacity.
    let mut market = unsafe { ctx.market_account.load_unchecked_mut() };

    let order_sector_index = {
        if is_bid {
            post_only_check_and_insert_order::<BidOrders>(&mut market, order)
        } else {
            post_only_check_and_insert_order::<AskOrders>(&mut market, order)
        }
    }?;

    Sector::check_in_bounds(market.sectors, user_sector_index_hint)?;
    // Find and verify the user's seat with the given index hint.
    // Safety: The index hint was just verified as in-bounds.
    let user_seat =
        load_mut_seat_with_hint(&mut market, user_sector_index_hint, ctx.user.address())?;

    let order_sector_index_bytes = order_sector_index.to_le_bytes();

    // 1. Check that the user has enough collateral to place the order and update their seat with
    //    the resulting decremented amount.
    // 2. Update the user seat's mapped order sectors. This also checks for duplicate prices so that
    //    all of a user's orders have a unique price.
    if is_bid {
        // 1. If the user is posting a bid, they intend to provide quote and receive base.
        user_seat.try_decrement_quote_available(quote_atoms)?;
        // 2. Add the order to the user's bids.
        user_seat
            .user_order_sectors
            .bids
            .add(&le_encoded_price, &order_sector_index_bytes)?;
    } else {
        // 1. If the user is posting an ask, they intend to provide base and receive quote.
        user_seat.try_decrement_base_available(base_atoms)?;
        // 2. Add the order to the user's asks.
        user_seat
            .user_order_sectors
            .asks
            .add(&le_encoded_price, &order_sector_index_bytes)?;
    }

    #[cfg(feature = "debug")]
    _event_buffer.add_to_buffer(
        PostOrderEventInstructionData::new(
            is_bid,
            user_sector_index_hint,
            order_sector_index,
            base_atoms,
            quote_atoms,
        ),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}

fn post_only_check_and_insert_order<T: OrdersCollection>(
    market: &mut MarketRefMut,
    order: Order,
) -> Result<SectorIndex, DropsetError> {
    T::post_only_crossing_check(&order, market)?;
    let (next_index, _hint) = T::find_new_order_next_index(market.orders::<T>().iter(), &order);
    insert_order(next_index, &mut market.orders::<T>(), order)
}
