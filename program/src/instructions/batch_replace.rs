//! See [`process_batch_replace`].

use dropset_interface::{
    self,
    error::{
        DropsetError,
        DropsetResult,
    },
    instructions::{
        BatchReplaceInstructionData,
        UnvalidatedOrders,
    },
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
            NIL,
        },
        user_order_sectors::{
            PriceToIndexEntry,
            MAX_ORDERS_USIZE,
        },
    },
};
use pinocchio::{
    account::AccountView,
    Address,
    ProgramResult,
};

use crate::{
    context::mutate_orders_context::MutateOrdersContext,
    shared::{
        order_operations::{
            insert_order,
            load_order_from_sector_index,
        },
        seat_operations::{
            load_mut_seat_with_hint,
            load_mut_seat_with_hint_unchecked,
            load_seat_with_hint,
        },
    },
};

/// Handler logic for batching multiple cancel + place order instructions in a single atomic
/// instruction.
///
/// # Safety
///
/// Since the accounts borrowed depend on the inner batch instructions, the most straightforward
/// safety contract is simply ensuring that **no Solana account data is currently borrowed** prior
/// to calling this instruction.
#[inline(never)]
pub unsafe fn process_batch_replace(
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let BatchReplaceInstructionData {
        user_sector_index_hint,
        new_bids,
        new_asks,
    } = BatchReplaceInstructionData::unpack_untagged(instruction_data)?;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts) }?;

    // Safety: Market account data isn't currently borrowed in any capacity.
    let mut market: MarketRefMut = unsafe { ctx.market_account.load_unchecked_mut() };

    Sector::check_in_bounds(market.sectors, user_sector_index_hint)?;

    // Safety: The user sector index hint was verified as in-bounds.
    unsafe {
        remove_orders_from_market_and_update_seat_balance::<BidOrders>(
            &mut market,
            ctx.user.address(),
            user_sector_index_hint,
        )?;

        remove_orders_from_market_and_update_seat_balance::<AskOrders>(
            &mut market,
            ctx.user.address(),
            user_sector_index_hint,
        )?;

        add_new_orders_and_update_seat_balance::<BidOrders>(
            &mut market,
            user_sector_index_hint,
            new_bids,
        )?;

        add_new_orders_and_update_seat_balance::<AskOrders>(
            &mut market,
            user_sector_index_hint,
            new_asks,
        )?;
    }

    Ok(())
}

/// Removes a user's orders from the market orders collection and update the seat balance to reflect
/// the collateral returned from closing those orders.
///
/// Note this does *not* remove the entries from the user seat's price -> order sectors mapping.
///
/// # Safety
///
/// Caller guarantees the user seat index passed is a non-NIL, valid, and in-bounds sector index.
#[inline(always)]
unsafe fn remove_orders_from_market_and_update_seat_balance<Side: OrdersCollection>(
    market: &mut MarketRefMut,
    user_address: &Address,
    valid_user_seat_index: SectorIndex,
) -> DropsetResult {
    // Find and verify the user's seat with the given index hint.
    // Safety: The index hint was just verified as in-bounds.
    let user_seat = load_seat_with_hint(market, valid_user_seat_index, user_address)?;
    let order_sectors = user_seat.user_order_sectors.order_sectors::<Side>();

    let mut collateral_returned: u64 = 0;

    for idx in order_sectors.to_sector_indices() {
        if idx != NIL {
            // Safety: Caller upholds the safety contract.
            let collateral_remaining =
                unsafe { load_order_from_sector_index(market, idx).collateral_amount::<Side>() };
            collateral_returned = collateral_returned
                .checked_add(collateral_remaining)
                .ok_or(DropsetError::ArithmeticOverflow)?;
            market.orders::<Side>().remove_at(idx);
        }
    }

    {
        let mut_user_seat = load_mut_seat_with_hint(market, valid_user_seat_index, user_address)?;
        mut_user_seat.try_increment_collateral_available::<Side>(collateral_returned)?;
    }

    Ok(())
}

/// First adds the passed orders to the appropriate market orders collection and user seat's price
/// -> order sectors mapping, skipping unnecessary checks and freeing unused entries in the order
/// sectors mapping where appropriate.
///
/// Then updates the user's seat balance to reflect the collateral necessary to post those orders.
///
/// # Safety
///
/// Caller guarantees the user seat index passed is a non-NIL, valid, and in-bounds sector index.
#[inline(always)]
unsafe fn add_new_orders_and_update_seat_balance<Side: OrdersCollection>(
    market: &mut MarketRefMut,
    valid_user_seat_index: SectorIndex,
    orders: UnvalidatedOrders,
) -> DropsetResult {
    let mut prev_price = Side::HIGHEST_PRIORITY_PRICE;
    let mut first_order = None;

    // Initialize hint to start from the head of the list
    let mut iter_sector_index = Side::head(market.header);

    let mut collateral_in_posted_orders: u64 = 0;

    let mut i = 0;
    for order_info in orders.into_valid_order_infos_iter() {
        let order = Order::new(order_info, valid_user_seat_index);
        let order_price = order.encoded_price();
        if i == 0 {
            // Clone the first order so it can be validated as a post-only order later.
            // There's no `prev_price` to meaningfully compare to since it's the first index,
            // so skip that check.
            first_order = Some(order.clone());
        } else {
            // Orders should be sorted in strictly descending price priority, meaning the previous
            // price should have a higher price priority than the current price.
            if !Side::has_higher_price_priority(&prev_price, &order_price) {
                return Err(DropsetError::OrdersNotSorted);
            }
        }

        prev_price = order.encoded_price();

        // Increase the collateral necessary to post.
        collateral_in_posted_orders = collateral_in_posted_orders
            .checked_add(order.collateral_amount::<Side>())
            .ok_or(DropsetError::ArithmeticOverflow)?;

        let list = &mut market.orders::<Side>();
        // Find the insertion point, continuing from where the previous search left off.
        // Safety: `iter_sector_idx` is either the head of the list (initially) or a sector index
        // returned from the previous call to `find_new_order_next_index`, which is guaranteed to be
        // either NIL or a valid sector index in the list.
        let (next_index, curr_index) =
            Side::find_new_order_next_index(unsafe { list.iter_from(iter_sector_index) }, &order);
        let insertion_index = insert_order(next_index, list, order)?;

        // Since orders are sorted in descending price priority, it's much more efficient to start
        // each search from the current index rather than from the head.
        iter_sector_index = curr_index;

        // Add the order to the user's order sectors mapping with an unchecked add operation where
        // the order isn't checked for duplication within the order sectors mapping.
        {
            // Safety: The seat hint was already validated as in-bounds. It could only possibly be
            // out of bounds now if the account data size was just reduced, which it was
            // not.
            let user_seat = load_mut_seat_with_hint_unchecked(market, valid_user_seat_index);
            // Safety: The duplication check isn't necessary below because the orders are guaranteed
            // to be strictly increasing or decreasing (through strictly decreasing price priority).
            // Therefore, there cannot be price duplicates and it's safe to directly mutate each
            // entry without going through the typical `.add(...)` API.
            unsafe {
                let entry_at_index_i = user_seat
                    .user_order_sectors
                    .order_sectors_mut::<Side>()
                    .get_mut_entry_at(i);

                *entry_at_index_i = PriceToIndexEntry {
                    encoded_price: order_price.into(),
                    sector_index: insertion_index.to_le_bytes(),
                };
            };
        }

        i += 1;
    }

    // Free the remaining price to order index entries in the user's seat.
    let user_seat = load_mut_seat_with_hint_unchecked(market, valid_user_seat_index);
    // Safety: `i` is the 0-based enumerated index over the valid order infos iterator. The number
    // of valid order infos is always <= `MAX_ORDERS_USIZE`, which means that `i`, after the
    // last increment is exactly the count of valid order infos. Thus `i <= MAX_ORDERS_USIZE`
    // and is exactly the index to begin removing mapped entries from.
    user_seat
        .user_order_sectors
        .order_sectors_mut::<Side>()
        .remove_range(i..MAX_ORDERS_USIZE);

    // Since these balance updates are reduced to a single operation, it's possible overflow or
    // underflow occurs where it normally wouldn't if the seat were updated for each order.
    // This is an acceptable trade-off since it's unlikely and removes multiple extra operations.
    // Since underflow is much more likely than overflow, add the returned collateral from the
    // canceled orders first, then subtract the necessary collateral from the posted orders.
    user_seat.try_decrement_collateral_available::<Side>(collateral_in_posted_orders)?;

    if let Some(first) = first_order {
        Side::post_only_crossing_check(&first, market)?;
    }

    Ok(())
}

#[cfg(feature = "debug")]
fn log_orders(
    bids: &dropset_interface::instructions::Orders,
    asks: &dropset_interface::instructions::Orders,
) {
    use crate::debug;

    debug!("num bids: {}", bids.num_orders);
    for order in bids
        .clone()
        .into_order_args_iter()
        .chain(asks.clone().into_order_args_iter())
    {
        debug!("OrderInfoArgs {");
        debug!("    price_mantissa: {}", order.price_mantissa);
        debug!("    base_scalar: {}", order.base_scalar);
        debug!("    base_exponent_biased: {}", order.base_exponent_biased);
        debug!("    quote_exponent_biased: {}", order.quote_exponent_biased);
        debug!("}");
        debug!("");
    }
}
