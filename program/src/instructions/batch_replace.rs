//! See [`process_batch_replace`].

use dropset_interface::{
    error::DropsetError,
    instructions::BatchReplaceInstructionData,
    state::{
        market::{
            MarketRef,
            MarketRefMut,
        },
        sector::{
            Sector,
            NIL,
        },
    },
};
use pinocchio::{
    account::AccountView,
    ProgramResult,
};
use price::to_order_info;

use crate::{
    context::mutate_orders_context::MutateOrdersContext,
    shared::{
        order_operations::load_mut_order_from_sector_index,
        seat_operations::find_seat_with_hint,
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

    let num_new_bids = new_bids.num_orders() as usize;
    let num_new_asks = new_asks.num_orders() as usize;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts)? };

    Ok(())
}

// ////////////////////////////////////////////////////////////////////////////////////////////// //
//                                            WIP below                                           //
// ////////////////////////////////////////////////////////////////////////////////////////////// //

/// Handler logic for batching multiple cancel + place order instructions in a single atomic
/// instruction.
///
/// # Safety
///
/// Since the accounts borrowed depend on the inner batch instructions, the most straightforward
/// safety contract is simply ensuring that **no Solana account data is currently borrowed** prior
/// to calling this instruction.
#[inline(never)]
pub unsafe fn process_batch_replace_WIP(
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let BatchReplaceInstructionData {
        user_sector_index_hint,
        new_bids,
        new_asks,
    } = BatchReplaceInstructionData::unpack_untagged(instruction_data)?;

    let num_new_bids = new_bids.num_orders() as usize;
    let num_new_asks = new_asks.num_orders() as usize;

    // Convert the order info args to order infos.
    let bid_infos = new_bids
        .into_order_args_iter()
        .map(|item| to_order_info(item).map_err(DropsetError::from));
    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts)? };

    #[cfg(feature = "debug")]
    log_orders(&new_bids, &new_asks);

    let (removed_bid_indices, removed_ask_indices) = {
        // Safety: Market account data isn't currently borrowed in any capacity.
        let market: MarketRef = unsafe { ctx.market_account.load_unchecked() };
        Sector::check_in_bounds(market.sectors, user_sector_index_hint)?;
        // Find and verify the user's seat with the given index hint.
        // Safety: The index hint was just verified as in-bounds.
        let user_seat = find_seat_with_hint(&market, user_sector_index_hint, ctx.user.address())?;

        let bid_sector_indices = user_seat.user_order_sectors.bids.to_sector_indices();
        let ask_sector_indices = user_seat.user_order_sectors.asks.to_sector_indices();
        (bid_sector_indices, ask_sector_indices)
    };

    let mut quote_from_canceled_bids: u64 = 0;
    let mut base_from_canceled_asks: u64 = 0;

    {
        for idx in removed_bid_indices {
            // Safety: Market account data isn't currently borrowed in any capacity.
            let mut market: MarketRefMut = unsafe { ctx.market_account.load_unchecked_mut() };
            if idx != NIL {
                // Safety: The sector index here is non-NIL and is a valid, in-bounds index from the
                // user's order sector mapping.
                unsafe {
                    let quote_remaining =
                        load_mut_order_from_sector_index(&mut market, idx).quote_remaining();
                    quote_from_canceled_bids = quote_from_canceled_bids
                        .checked_add(quote_remaining)
                        .ok_or(DropsetError::ArithmeticOverflow)?;
                    market.bids().remove_at(idx);
                }
            }
        }
    }
    {
        for idx in removed_ask_indices {
            // Safety: Market account data isn't currently borrowed in any capacity.
            let mut market: MarketRefMut = unsafe { ctx.market_account.load_unchecked_mut() };
            if idx != NIL {
                // Safety: The sector index here is non-NIL and is a valid, in-bounds index from the
                // user's order sector mapping.
                unsafe {
                    let base_remaining =
                        load_mut_order_from_sector_index(&mut market, idx).base_remaining();
                    base_from_canceled_asks = base_from_canceled_asks
                        .checked_add(base_remaining)
                        .ok_or(DropsetError::ArithmeticOverflow)?;
                    market.asks().remove_at(idx);
                }
            }
        }
    }

    // 1. Remove all `!.is_free()` orders in user order sectors. After this, each order `.is_free()`
    //    - Remove/free each order (from user order sectors)
    //    - Push each removed sector index to `removed_order_indices`

    // 2. Iterate over the `removed_order_indices`, and for each:
    //    - Remove each order (from the market bids or asks)
    //    - Increment collateral returned (transferred from market -> user):
    //      - `base_from_canceled_orders` (asks)
    //      - `quote_from_canceled_orders` (bids)
    //
    // 3. Iterate over the new orders and for each:
    //    - Only for the *first* order: do a `post_only_crossing_check`. The rest aren't necessary
    //      because the orders are guaranteed to be sorted according to the book side.
    //    - Ensure that the order is sorted properly and strictly increasing/decreasing (wrt side)
    //    - Increment collateral necessary (transferred from user -> market):
    //      - `base_in_posted` (asks)
    //      - `quote_in_posted` (quote)
    //    - Post each order to the appropriate orders collection; don't search the whole collection
    //      for the insertion index; start from the last order insertion index since they're sorted.
    //    - Push the (insertion_index, price) tuple to `new_order_indices`
    //
    // 4. For each (insertion_index, price) in `new_order_indices`:
    //    - Add to the user order sectors mapping
    //    - Do it with an unchecked add; i.e., skip the duplicate check, because it's guaranteed the
    //      order prices are strictly increasing or decreasing and thus cannot be duplicates.
    //
    // 5. Adjust the user's seat balances:
    //    - base = new_base.checked_add(base_from_canceled_orders).checked_sub(base_in_posted)
    //    - quote = new_quote.checked_add(quote_from_canceled_orders).checked_sub(quote_in_posted)

    Ok(())
}

// #[inline(always)]
// fn remove_orders(
//     new_orders: Orders,
//     order_sectors: &mut OrderSectors,
// ) -> (usize, [SectorIndex; MAX_ORDERS_USIZE]) {
//     let num_new = new_orders.num_orders;
//     let new_infos = new_orders
//         .into_order_args_iter()
//         .map(|item| to_order_info(item).map_err(DropsetError::from));
//     let mut removed = [MaybeUninit::<SectorIndex>::uninit(); MAX_ORDERS_USIZE];
//     let ptr = removed.as_mut_ptr() as *mut SectorIndex;

//     // Store each removed index by iterating over the new order infos.
//     for (i, info) in new_infos.enumerate() {
//         let idx = order_sectors.remove(info?.encoded_price.as_u32())?;

//         removed.add(i).write(SectorIndex::from_le_bytes(idx));
//     }

//     // Zero out the rest of the array, since it will never be read.
//     removed
//         .add(num_new)
//         .write_bytes(0u8, MAX_ORDERS_USIZE.unchecked_sub(num_new));
// }

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
