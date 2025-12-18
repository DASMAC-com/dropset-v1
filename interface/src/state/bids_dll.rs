//! Doubly linked list of bid order nodes with [`crate::state::order::Order`] payloads.

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        linked_list::{
            LinkedList,
            LinkedListOperations,
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

pub struct BidOrders;

impl OrdersCollection for BidOrders {
    /// Bids are inserted in descending order. The top of the book (first price on the book) is thus
    /// the highest price.
    ///
    /// Inserting a new bid at an existing price has the lowest time order precedence among all bids
    /// of that price, so in order to find the insertion index for a new bid, find the first price
    /// that is less than the new bid and insert before it.
    ///
    /// If the bid is the lowest price on the book, it's inserted at the end.
    #[inline(always)]
    fn find_new_order_next_index<T: OrdersCollection + LinkedListOperations>(
        list: &LinkedList<'_, T>,
        new_order: &Order,
    ) -> SectorIndex {
        // Find the first price that is less than the new bid.
        for (index, node) in list.iter() {
            let order = node.load_payload::<Order>();
            if order.encoded_price() < new_order.encoded_price() {
                return index;
            }
        }

        // If the node is to be inserted at the end of the list, the new `next` index is `NIL`,
        // since the new node is the new tail.
        NIL
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
        let first_ask_node = market.iter_asks().next();
        match first_ask_node {
            // Check that the bid price is less than the first ask (the lowest ask).
            Some((_idx, ask_node)) => {
                let lowest_ask = ask_node.load_payload::<Order>();
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
}

pub type BidOrdersLinkedList<'a> = LinkedList<'a, BidOrders>;

/// Operations for the sorted, doubly linked list of nodes containing bid
/// [`crate::state::order::Order`] payloads.
impl LinkedListOperations for BidOrders {
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

    fn increment_num_nodes(header: &mut MarketHeader) {
        header.increment_num_bids();
    }

    fn decrement_num_nodes(header: &mut MarketHeader) {
        header.decrement_num_bids();
    }
}
