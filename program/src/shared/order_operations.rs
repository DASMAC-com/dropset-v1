//! Core logic for manipulating and traversing [`Order`]s in the [`OrdersLinkedList`].

use dropset_interface::{
    error::DropsetError,
    state::{
        market::{
            MarketRef,
            MarketRefMut,
        },
        node::Node,
        order::Order,
        orders_dll::OrdersLinkedList,
        sector::{
            SectorIndex,
            NIL,
        },
    },
};

/// Insert a new user order into the orders collection.
///
/// NOTE: this function solely inserts the order into the orders collection. It doesn't update the
/// user's seat nor does it check for duplicate prices posted by the same user.
pub fn insert_order(
    list: &mut OrdersLinkedList,
    order: Order,
    is_bid: bool,
) -> Result<SectorIndex, DropsetError> {
    let sector_index = {
        let next_index = find_new_order_next_index(list, &order, is_bid);
        let order_bytes = order.as_bytes();

        if next_index == list.header.orders_dll_head() {
            list.push_front(order_bytes)
        } else if next_index == NIL {
            list.push_back(order_bytes)
        } else {
            // Safety: The index used here was returned by the iterator so it must be in-bounds.
            unsafe { list.insert_before(next_index, order_bytes) }
        }
    }?;

    Ok(sector_index)
}

/// Bids are inserted in descending order so that the top of the book (first price on the book) is
/// the highest price.
///
/// Inserting a new bid at an existing price has the lowest time order precedence among all bids of
/// that price, so in order to find the insertion index for a new bid, find the first price that is
/// less than the new bid and insert before it.
///
/// If the bid is the lowest price on the book, it's inserted at the end.
///
/// For asks, the logic is simply inverted: asks are inserted in ascending order so that the top of
/// the book is the lowest price. To find the insertion index, find the first price that is greater
/// than the new bid and insert before it.
///
/// If the ask is the highest price on the book, it's inserted at the end.
///
/// This function returns the new prev and next indices for the new node. Thus the list would be
/// updated from this:
///
/// prev => next
///
/// To this:
///
/// prev => new => next
///
/// where this function returns the `next` node's sector index.
#[inline(always)]
fn find_new_order_next_index(
    list: &OrdersLinkedList,
    new_order: &Order,
    is_bid: bool,
) -> SectorIndex {
    let new_encoded_price = new_order.encoded_price();
    if is_bid {
        // Find the first price that is less than the new bid.
        for (index, node) in list.iter() {
            let order = node.load_payload::<Order>();
            if order.encoded_price() < new_encoded_price {
                return index;
            }
        }
    } else {
        // Find the first price that is greater than the new bid.
        for (index, node) in list.iter() {
            let order = node.load_payload::<Order>();
            if order.encoded_price() > new_encoded_price {
                return index;
            }
        }
    }

    // If the node is to be inserted at the end of the list, the new `next` index is `NIL`, since
    // the new node is the new tail.
    NIL
}

/// Converts a sector index to a mutable order given a sector index.
///
/// Caller should ensure that `validated_sector_index` is indeed a sector index pointing to a valid
/// order.
///
/// # Safety
///
/// Caller guarantees `validated_sector_index` is in-bounds of `market.sectors` bytes.
pub unsafe fn load_mut_order_from_sector_index<'a>(
    market: MarketRefMut<'a>,
    validated_sector_index: SectorIndex,
) -> &'a mut Order {
    // Safety: Caller guarantees 'validated_sector_index' is in-bounds.
    let node = unsafe { Node::from_sector_index_mut(market.sectors, validated_sector_index) };

    // Safety: All bit patterns are valid for order structs.
    unsafe { node.load_payload_mut::<Order>() }
}
