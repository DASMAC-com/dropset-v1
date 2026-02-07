//! Doubly linked list of ask order sectors with [`crate::state::order::Order`] payloads.

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        linked_list::{
            LinkedList,
            LinkedListHeaderOperations,
        },
        market::Market,
        market_header::MarketHeader,
        order::{
            Order,
            OrdersCollection,
        },
        sector::{
            SectorIndex,
            NIL,
        },
    },
};

pub struct AskOrders;

impl OrdersCollection for AskOrders {
    /// Asks are inserted in ascending order. The top of the book (first price on the book) is thus
    /// the lowest price.
    ///
    /// Inserting a new ask at an existing price has the lowest time order precedence among all asks
    /// of that price, so in order to find the insertion index for a new ask, find the first price
    /// that is greater than the new ask and insert before it.
    ///
    /// If the ask is the highest price on the book, it's inserted at the end.
    #[inline(always)]
    fn find_new_order_next_index<T: OrdersCollection + LinkedListHeaderOperations>(
        list: &LinkedList<'_, T>,
        new_order: &Order,
    ) -> SectorIndex {
        // Find the first price that is greater than the new ask.
        for (index, sector) in list.iter() {
            let order = sector.load_payload::<Order>();
            if order.encoded_price() > new_order.encoded_price() {
                return index;
            }
        }

        // If the sector is to be inserted at the end of the list, the new `next` index is `NIL`,
        // since the new sector is the new tail.
        NIL
    }

    /// A post-only ask order can only be posted if the input price > the highest bid, because it
    /// would immediately take otherwise.
    ///
    /// If this condition is satisfied or if the bid side is empty, the order cannot cross and may
    /// be posted.
    #[inline(always)]
    fn post_only_crossing_check<H, S>(order: &Order, market: &Market<H, S>) -> DropsetResult
    where
        H: AsRef<MarketHeader>,
        S: AsRef<[u8]>,
    {
        let ask_price = order.encoded_price();
        let first_bid_sector = market.iter_bids().next();
        match first_bid_sector {
            // Check that the ask wouldn't immediately take (and is thus post only) by ensuring its
            // price is greater than the first/highest bid.
            Some((_idx, bid_sector)) => {
                let highest_bid = bid_sector.load_payload::<Order>();
                if ask_price > highest_bid.encoded_price() {
                    Ok(())
                } else {
                    Err(DropsetError::PostOnlyWouldImmediatelyFill)
                }
            }
            // There are no bid orders, so the ask cannot cross and may be posted.
            None => Ok(()),
        }
    }
}

pub type AskOrdersLinkedList<'a> = LinkedList<'a, AskOrders>;

/// Operations for the sorted, doubly linked list of sectors containing ask
/// [`crate::state::order::Order`] payloads.
impl LinkedListHeaderOperations for AskOrders {
    fn head(header: &MarketHeader) -> SectorIndex {
        header.asks_dll_head()
    }

    fn set_head(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_asks_dll_head(new_index);
    }

    fn tail(header: &MarketHeader) -> SectorIndex {
        header.asks_dll_tail()
    }

    fn set_tail(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_asks_dll_tail(new_index);
    }

    fn increment_num_sectors(header: &mut MarketHeader) {
        header.increment_num_asks();
    }

    fn decrement_num_sectors(header: &mut MarketHeader) {
        header.decrement_num_asks();
    }
}
