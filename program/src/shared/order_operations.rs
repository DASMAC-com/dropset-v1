//! Core logic for manipulating and traversing [`Order`]s in the [`OrdersLinkedList`].

use dropset_interface::{
    error::DropsetError,
    state::{
        linked_list::{
            LinkedList,
            LinkedListOperations,
        },
        market::MarketRef,
        node::Node,
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

/// Insert a new user order into the orders collection.
///
/// NOTE: this function solely inserts the order into the orders collection. It doesn't update the
/// user's seat nor does it check for duplicate prices posted by the same user.
pub fn insert_order<T: OrdersCollection + LinkedListOperations>(
    list: &mut LinkedList<'_, T>,
    order: Order,
) -> Result<SectorIndex, DropsetError> {
    let sector_index = {
        let next_index = T::find_new_order_next_index(list, &order);
        let order_bytes = order.as_bytes();

        if next_index == T::head(list.header) {
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

/// Converts a sector index to an order given a sector index.
///
/// Caller should ensure that `validated_sector_index` is indeed a sector index pointing to a valid
/// order.
///
/// # Safety
///
/// Caller guarantees `validated_sector_index` is in-bounds of `market.sectors` bytes.
pub unsafe fn load_order_from_sector_index(
    market: MarketRef<'_>,
    validated_sector_index: SectorIndex,
) -> &'_ Order {
    // Safety: Caller guarantees 'validated_sector_index' is in-bounds.
    let node = unsafe { Node::from_sector_index(market.sectors, validated_sector_index) };
    node.load_payload::<Order>()
}
