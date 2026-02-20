//! Doubly linked list of bid order sectors with [`crate::state::order::Order`] payloads.

use price::EncodedPrice;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        linked_list::{
            LinkedList,
            LinkedListHeaderOperations,
            LinkedListIter,
        },
        market::Market,
        market_header::MarketHeader,
        market_seat::MarketSeat,
        order::{
            NextSectorIndex,
            Order,
            OrdersCollection,
        },
        sector::{
            SectorIndex,
            NIL,
        },
        user_order_sectors::{
            OrderSectors,
            UserOrderSectors,
        },
    },
};

pub struct BidOrders;

impl OrdersCollection for BidOrders {
    const HIGHEST_PRIORITY_PRICE: EncodedPrice = EncodedPrice::infinity();

    /// Bids are inserted in descending order. The top of the book (first price on the book) is thus
    /// the highest price.
    ///
    /// Inserting a new bid at an existing price has the lowest time order precedence among all bids
    /// of that price, so in order to find the insertion index for a new bid, find the first price
    /// that is less than the new bid and insert before it.
    ///
    /// If the bid is the lowest price on the book, it's inserted at the end.
    #[inline(always)]
    fn find_new_order_next_index(
        mut list_iterator: LinkedListIter<'_>,
        new_order: &Order,
    ) -> (NextSectorIndex, SectorIndex) {
        // Find the first price that is less than the new bid.
        for (index, sector) in list_iterator.by_ref() {
            let order = sector.load_payload::<Order>();
            if order.encoded_price() < new_order.encoded_price() {
                return (NextSectorIndex(index), list_iterator.curr);
            }
        }

        // If the sector is to be inserted at the end of the list, the new `next` index is `NIL`,
        // since the new sector is the new tail.
        (NextSectorIndex(NIL), list_iterator.curr)
    }

    /// A post-only bid order can only be posted if the input price < the lowest ask, because it
    /// would immediately take otherwise.
    ///
    /// If this condition is satisfied or if the ask side is empty, the order cannot cross and may
    /// be posted.
    #[inline(always)]
    fn post_only_crossing_check<H, S>(order: &Order, market: &Market<H, S>) -> DropsetResult
    where
        H: AsRef<MarketHeader>,
        S: AsRef<[u8]>,
    {
        let bid_price = order.encoded_price();
        let first_ask_sector = market.iter_asks().next();
        match first_ask_sector {
            // Check that the bid wouldn't immediately take (and is thus post only) by ensuring its
            // price is less than the first/lowest ask.
            Some((_idx, ask_sector)) => {
                let lowest_ask = ask_sector.load_payload::<Order>();
                if bid_price < lowest_ask.encoded_price() {
                    Ok(())
                } else {
                    Err(DropsetError::PostOnlyWouldImmediatelyFill)
                }
            }
            // There are no ask orders, so the bid cannot cross and may be posted.
            None => Ok(()),
        }
    }

    /// Users put up quote as collateral when posting bids.
    ///
    /// Returns the quote remaining in an bid.
    #[inline(always)]
    fn get_order_collateral(order: &Order) -> u64 {
        order.quote_remaining()
    }

    /// Tries to decrement the quote available in a user's seat (bid collateral is quote).
    #[inline(always)]
    fn try_decrement_seat_collateral_available(
        seat: &mut MarketSeat,
        amount: u64,
    ) -> DropsetResult {
        seat.try_decrement_quote_available(amount)
    }

    /// Tries to increment the quote available in a user's seat (bid collateral is quote).
    #[inline(always)]
    fn try_increment_seat_collateral_available(
        seat: &mut MarketSeat,
        amount: u64,
    ) -> DropsetResult {
        seat.try_increment_quote_available(amount)
    }

    #[inline(always)]
    fn get_order_sectors(user_order_sectors: &UserOrderSectors) -> &OrderSectors {
        &user_order_sectors.bids
    }

    #[inline(always)]
    fn get_mut_order_sectors(user_order_sectors: &mut UserOrderSectors) -> &mut OrderSectors {
        &mut user_order_sectors.bids
    }

    /// Orders with higher prices are closer to the top of book on the bid side, so they have a
    /// higher price priority; i.e., they are inserted and thus filled before orders with lower
    /// prices.
    #[inline(always)]
    fn has_higher_price_priority(a: &EncodedPrice, b: &EncodedPrice) -> bool {
        a.has_higher_bid_priority(b)
    }
}

pub type BidOrdersLinkedList<'a> = LinkedList<'a, BidOrders>;

/// Operations for the sorted, doubly linked list of sectors containing bid
/// [`crate::state::order::Order`] payloads.
impl LinkedListHeaderOperations for BidOrders {
    fn head(header: &MarketHeader) -> SectorIndex {
        header.bids_dll_head()
    }

    fn set_head(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_bids_dll_head(new_index);
    }

    fn tail(header: &MarketHeader) -> SectorIndex {
        header.bids_dll_tail()
    }

    fn set_tail(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_bids_dll_tail(new_index);
    }

    fn increment_num_elements(header: &mut MarketHeader) {
        header.increment_num_bids();
    }

    fn decrement_num_elements(header: &mut MarketHeader) {
        header.decrement_num_bids();
    }
}
