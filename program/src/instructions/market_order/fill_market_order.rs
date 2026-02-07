use core::num::NonZeroU64;

use dropset_interface::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        asks_dll::AskOrders,
        bids_dll::BidOrders,
        linked_list::LinkedListHeaderOperations,
        market_seat::MarketSeat,
        sector::{
            Sector,
            SectorIndex,
            NIL,
        },
    },
};
use pinocchio::hint;
use price::EncodedPrice;

use crate::{
    context::market_order_context::MarketOrderContext,
    instructions::market_order::mul_div_checked,
    shared::order_operations::{
        load_mut_order_from_sector_index,
        load_order_from_sector_index,
    },
};

struct OrderSnapshot {
    base_remaining: u64,
    quote_remaining: u64,
    encoded_price: u32,
    maker_seat_sector: SectorIndex,
    order_sector: SectorIndex,
}

impl OrderSnapshot {
    #[inline(always)]
    const fn get_constrained_remaining<const BASE_DENOM: bool>(&self) -> u64 {
        if BASE_DENOM {
            self.base_remaining
        } else {
            self.quote_remaining
        }
    }

    #[inline(always)]
    const fn get_counter_asset_remaining<const BASE_DENOM: bool>(&self) -> u64 {
        if BASE_DENOM {
            self.quote_remaining
        } else {
            self.base_remaining
        }
    }
}

pub struct AmountsFilled {
    pub base: u64,
    pub quote: u64,
}

/// `IS_BUY` determines whether or not it's a market buy or a market sell.
///
/// `BASE_DENOM` determines which asset the constraint input amount is in.
/// That is, if the user specifies they want to market buy 1000 quote atoms worth:
///
/// `IS_BUY == true && BASE_DENOM == false && amount == 1000`
///
/// This function returns the amounts filled denominated in both base and quote. The ratio of these
/// two values is effectively the average fill price.
///
/// # Safety
///
/// The market account data must not be currently borrowed.
#[inline(always)]
pub unsafe fn fill_market_order<const IS_BUY: bool, const BASE_DENOM: bool>(
    ctx: &'_ mut MarketOrderContext<'_>,
    order_size: u64,
) -> Result<AmountsFilled, DropsetError> {
    // All amounts in this function are in atoms.
    let mut constraint_asset_remaining = order_size;
    let mut counter_asset_filled: u64 = 0;

    // Iterate over each order on the book, filling each posted order in whole as long as the
    // market order has any remaining size.
    // That is, as long as the amount not filled yet exceeds the amount in the next posted order,
    // simply close the order and decrement the remaining amount by the amount used to fill the
    // order. This skips muldiv operations until the very last partial fill.
    while let Some(top_order) = top_of_book_snapshot::<IS_BUY>(ctx) {
        // If there's nothing left to fill, break from the loop. The last order filled cleanly with
        // no remainder so there's no partial order to fill.
        if hint::unlikely(constraint_asset_remaining == 0) {
            break;
        } else {
            // Safety:
            // 1. Market account data isn't currently borrowed per this function's safety contract.
            // 2. The head/top of book order sector index is valid.
            // 3. The market maker's seat order sector index is valid.
            unsafe {
                // Otherwise, check if this should be a partial fill or a full fill.
                // If the constrained asset amount remaining in the top order is <= to the amount
                // that should be filled for the taker, it's a full fill. That is, the maker order
                // can be completely filled and thus removed from the books.
                if top_order.get_constrained_remaining::<BASE_DENOM>() <= constraint_asset_remaining
                {
                    // Safety: The order's constrained amount remaining is <= the constraint asset
                    // remaining.
                    full_fill::<IS_BUY, BASE_DENOM>(
                        ctx,
                        &mut constraint_asset_remaining,
                        &mut counter_asset_filled,
                        &top_order,
                    )?;

                    // Safety: The market account data isn't currently borrowed and the top order's
                    // maker seat sector index still points to a valid seat in memory.
                    #[cfg(debug_assertions)]
                    ensure_order_has_been_removed::<IS_BUY>(ctx, &top_order);
                } else {
                    // Otherwise, it's a partial fill. That is, the maker order *cannot* be
                    // completely filled and must be mutated to reflect the new amounts remaining.
                    partial_fill::<IS_BUY, BASE_DENOM>(
                        ctx,
                        &mut constraint_asset_remaining,
                        &mut counter_asset_filled,
                        &top_order,
                    )?;

                    // The taker order amount should be completely filled now.
                    debug_assert_eq!(constraint_asset_remaining, 0);
                    break;
                }
            }
        }
    }

    // Safety: The constraint asset remaining never increments, so it's always <= the order size.
    let constrained_asset_filled = order_size.unchecked_sub(constraint_asset_remaining);

    if BASE_DENOM {
        Ok(AmountsFilled {
            base: constrained_asset_filled,
            quote: counter_asset_filled,
        })
    } else {
        Ok(AmountsFilled {
            base: counter_asset_filled,
            quote: constrained_asset_filled,
        })
    }
}

#[inline(always)]
fn top_of_book_snapshot<const IS_BUY: bool>(ctx: &'_ MarketOrderContext) -> Option<OrderSnapshot> {
    // Safety: Scoped borrow of the market account data to check the top of book.
    let market = unsafe { ctx.market_account.load_unchecked() };

    let head_index = if IS_BUY {
        AskOrders::head(market.header)
    } else {
        BidOrders::head(market.header)
    };

    if head_index == NIL {
        None
    } else {
        // Safety: The head index is a non-NIL sector index pointing to a valid order sector.
        let order = unsafe { load_order_from_sector_index(&market, head_index) };
        Some(OrderSnapshot {
            base_remaining: order.base_remaining(),
            quote_remaining: order.quote_remaining(),
            encoded_price: order.encoded_price(),
            maker_seat_sector: order.user_seat(),
            order_sector: head_index,
        })
    }
}

/// Fully fill the order, by doing the following:
/// 1. Remove the order from the orders collection.
/// 2. Update the filled maker seat's balance and remove the order from the maker seat's price to
///    order map.
/// 3. Update the constraint asset remaining and the counter asset filled.
///
/// # Safety
///
/// The market account data must not be currently borrowed and the top order sector index and the
/// user seat sector index must both still point to valid, properly typed sectors in memory.
///
/// The constraint asset remaining must be <= the top order's constraint asset remaining.
#[inline(always)]
unsafe fn full_fill<const IS_BUY: bool, const BASE_DENOM: bool>(
    ctx: &'_ mut MarketOrderContext<'_>,
    constraint_asset_remaining: &mut u64,
    counter_asset_filled: &mut u64,
    top_order: &OrderSnapshot,
) -> DropsetResult {
    // 1. Close/remove the order from the orders collection.
    if IS_BUY {
        ctx.market_account
            .load_unchecked_mut()
            .asks()
            .remove_at(top_order.order_sector);
    } else {
        ctx.market_account
            .load_unchecked_mut()
            .bids()
            .remove_at(top_order.order_sector);
    }

    // 2. Update the filled maker seat's balance and remove the order from their price to order
    // sector map.
    // Safety: The safety contract is essentially a subset of the calling function.
    unsafe {
        update_maker_seat_after_fill::<IS_BUY, false>(
            ctx,
            top_order.maker_seat_sector,
            // The base/quote amount filled is simply the (now previously) top order's amounts
            // remaining, since this was a full fill.
            top_order.base_remaining,
            top_order.quote_remaining,
            top_order.encoded_price,
        )
    }?;

    // 3. Update the constrained amount not filled yet and the counter asset total filled.
    // Safety: The amount of constraint asset remaining must be >= the denominated constrained
    // amount in the top order or this would not be a full fill.
    *constraint_asset_remaining = constraint_asset_remaining
        .unchecked_sub(top_order.get_constrained_remaining::<BASE_DENOM>());

    *counter_asset_filled = counter_asset_filled
        .checked_add(top_order.get_counter_asset_remaining::<BASE_DENOM>())
        .ok_or(DropsetError::ArithmeticOverflow)?;

    Ok(())
}

#[inline(always)]
fn partial_fill<const IS_BUY: bool, const BASE_DENOM: bool>(
    ctx: &'_ mut MarketOrderContext<'_>,
    constraint_asset_remaining: &mut u64,
    counter_asset_filled: &mut u64,
    top_order: &OrderSnapshot,
) -> DropsetResult {
    let remaining_constrained_asset_in_top_order =
        dropset_non_zero_u64(top_order.get_constrained_remaining::<BASE_DENOM>())?;
    let remaining_counter_asset_in_top_order =
        top_order.get_counter_asset_remaining::<BASE_DENOM>();

    let partial_counter_asset_fill_amount = mul_div_checked(
        *constraint_asset_remaining,
        remaining_counter_asset_in_top_order,
        remaining_constrained_asset_in_top_order,
    )?;

    // Add the partial fill amount to the total counter asset filled.
    *counter_asset_filled = counter_asset_filled
        .checked_add(partial_counter_asset_fill_amount)
        .ok_or(DropsetError::ArithmeticOverflow)?;

    let (base_filled, quote_filled) = {
        // Now update the order to reflect the new remaining amounts after the partial fill.
        // Safety: Scoped mutable borrow of the market account data.
        let mut market = unsafe { ctx.market_account.load_unchecked_mut() };

        // Safety: The order sector index is non-NIL and pointing to a valid order sector.
        let order =
            unsafe { load_mut_order_from_sector_index(&mut market, top_order.order_sector) };

        #[rustfmt::skip]
        let (base_filled, quote_filled) = if BASE_DENOM {
            (*constraint_asset_remaining, partial_counter_asset_fill_amount)
        } else {
            (partial_counter_asset_fill_amount, *constraint_asset_remaining)
        };

        // Safety: The amount not filled yet for both sides is always <= the amount in the top
        // order, otherwise this would not be a partial fill.
        unsafe {
            let new_base = top_order.base_remaining.unchecked_sub(base_filled);
            let new_quote = top_order.quote_remaining.unchecked_sub(quote_filled);
            order.set_base_remaining(new_base);
            order.set_quote_remaining(new_quote);
        };

        (base_filled, quote_filled)
    };

    // Set the remaining amount not yet filled to zero.
    *constraint_asset_remaining = 0;

    // Update the maker's seat to reflect the partial fill.
    // Safety: The market account data is not currently borrowed and the maker's user seat inside
    // the top order still points to a valid user.
    unsafe {
        update_maker_seat_after_fill::<IS_BUY, true>(
            ctx,
            top_order.maker_seat_sector,
            base_filled,
            quote_filled,
            top_order.encoded_price,
        )
    }?;

    Ok(())
}

#[inline(always)]
fn dropset_non_zero_u64(value: u64) -> Result<NonZeroU64, DropsetError> {
    if value == 0 {
        Err(DropsetError::AmountCannotBeZero)
    } else {
        // Safety: The value was just verified as non-zero.
        Ok(unsafe { NonZeroU64::new_unchecked(value) })
    }
}

/// # Safety
///
/// The market account data must not be currently borrowed and the passed order's maker seat sector
/// index must still point to a valid seat in memory.
#[inline(always)]
unsafe fn update_maker_seat_after_fill<const IS_BUY: bool, const PARTIAL_FILL: bool>(
    ctx: &'_ mut MarketOrderContext<'_>,
    maker_seat_sector: SectorIndex,
    base_filled: u64,
    quote_filled: u64,
    encoded_price: u32,
) -> DropsetResult {
    // Safety: Single, scoped mutable borrow of the market account data.
    let market = ctx.market_account.load_unchecked_mut();
    // Safety: The user seat sector index is in-bounds, as it came from the order.
    let sector = unsafe { Sector::from_sector_index_mut(market.sectors, maker_seat_sector) };
    let maker_seat = sector.load_payload_mut::<MarketSeat>();
    if IS_BUY {
        // Market buy means a maker's ask got filled, so they receive quote.
        maker_seat.try_increment_quote_available(quote_filled)?;

        // If it's a complete/full fill, remove the order sector index from the price to index map.
        if !PARTIAL_FILL {
            maker_seat.user_order_sectors.asks.remove(encoded_price)?;
        }
    } else {
        // Market sell means a maker's bid got filled, so they receive base.
        maker_seat.try_increment_base_available(base_filled)?;

        // If it's a complete/full fill, remove the order sector index from the price to index map.
        if !PARTIAL_FILL {
            maker_seat.user_order_sectors.bids.remove(encoded_price)?;
        }
    }

    Ok(())
}

/// # Safety
///
/// The market account data must not be currently borrowed and top order's maker seat sector index
/// must still point to a valid seat in memory.
#[cfg(debug_assertions)]
unsafe fn ensure_order_has_been_removed<const IS_BUY: bool>(
    ctx: &'_ MarketOrderContext,
    top_order: &OrderSnapshot,
) {
    use price::LeEncodedPrice;

    // Safety: Single, scoped mutable borrow of the market account data.
    let market = ctx.market_account.load_unchecked();
    // Safety: The user seat sector index is in-bounds, as it came from the order.
    let sector = unsafe { Sector::from_sector_index(market.sectors, top_order.maker_seat_sector) };
    let maker_seat = sector.load_payload::<MarketSeat>();
    let encoded_price: EncodedPrice = top_order
        .encoded_price
        .try_into()
        .expect("Should be a valid encoded price");
    let le_encoded_price: &LeEncodedPrice = &encoded_price.into();

    let orders = if IS_BUY {
        &maker_seat.user_order_sectors.asks
    } else {
        &maker_seat.user_order_sectors.bids
    };

    debug_assert!({ orders.get(le_encoded_price).is_none() });
}
