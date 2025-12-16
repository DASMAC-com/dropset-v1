//! Core logic for manipulating and traversing [`MarketSeat`]s.

use dropset_interface::{
    error::DropsetError,
    state::{
        market::{
            MarketRef,
            MarketRefMut,
        },
        market_seat::MarketSeat,
        node::Node,
        seats_dll::SeatsLinkedList,
        sector::{
            SectorIndex,
            NIL,
        },
    },
};
use pinocchio::pubkey::{
    pubkey_eq,
    Pubkey,
};

pub fn try_insert_market_seat(
    list: &mut SeatsLinkedList,
    seat: MarketSeat,
) -> Result<SectorIndex, DropsetError> {
    let (prev_index, next_index) = find_new_seat_prev_and_next(list, &seat.user);
    let seat_bytes = seat.as_bytes();

    // Return an error early if the user already exists in the seat list at the previous index.
    if prev_index != NIL {
        // Safety: `prev_index` is non-NIL and was returned by an iterator, so it must be in-bounds.
        let prev_node = unsafe { Node::from_sector_index(list.sectors, prev_index) };
        let prev_seat = prev_node.load_payload::<MarketSeat>();
        if pubkey_eq(&seat.user, &prev_seat.user) {
            return Err(DropsetError::UserAlreadyExists);
        }
    }

    if next_index == list.header.seats_dll_head() {
        list.push_front(seat_bytes)
    } else if next_index == NIL {
        list.push_back(seat_bytes)
    } else {
        // Safety: The index used here was returned by the iterator so it must be in-bounds.
        unsafe { list.insert_before(next_index, seat_bytes) }
    }
}

/// This function returns the new prev and next indices for the new node. Thus the list would be
/// updated from this:
///
/// prev => next
///
/// To this:
///
/// prev => new => next
///
/// where this function returns `(prev, next)` as sector indices.
#[inline(always)]
fn find_new_seat_prev_and_next(
    list: &SeatsLinkedList,
    user: &Pubkey,
) -> (SectorIndex, SectorIndex) {
    for (index, node) in list.iter() {
        let seat = node.load_payload::<MarketSeat>();
        if user < &seat.user {
            return (node.prev(), index);
        }
    }
    // If the node is to be inserted at the end of the list, the new `prev` is the current tail
    // and the new `next` is `NIL`, since the new node is the new tail.
    (list.header.seats_dll_tail(), NIL)
}

/// Tries to find a market seat given an index hint.
///
/// # Safety
///
/// Caller guarantees `hint` is in-bounds of `market.sectors` bytes.
pub unsafe fn find_seat_with_hint<'a>(
    market: MarketRef<'a>,
    hint: SectorIndex,
    user: &Pubkey,
) -> Result<&'a MarketSeat, DropsetError> {
    // Safety: Caller guarantees `hint` is in-bounds.
    let node = unsafe { Node::from_sector_index(market.sectors, hint) };
    let seat = node.load_payload::<MarketSeat>();
    if pubkey_eq(user, &seat.user) {
        Ok(seat)
    } else {
        Err(DropsetError::InvalidIndexHint)
    }
}

/// Tries to find a mutable market seat given an index hint.
///
/// # Safety
///
/// Caller guarantees `hint` is in-bounds of `market.sectors` bytes.
pub unsafe fn find_mut_seat_with_hint<'a>(
    market: MarketRefMut<'a>,
    hint: SectorIndex,
    user: &Pubkey,
) -> Result<&'a mut MarketSeat, DropsetError> {
    // Safety: Caller guarantees `hint` is in-bounds.
    let node = unsafe { Node::from_sector_index_mut(market.sectors, hint) };
    let seat = node.load_payload_mut::<MarketSeat>();
    if pubkey_eq(user, &seat.user) {
        Ok(seat)
    } else {
        Err(DropsetError::InvalidIndexHint)
    }
}
