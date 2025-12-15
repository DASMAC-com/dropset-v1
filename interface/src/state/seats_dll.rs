//! Doubly linked list utilities for traversing, inserting, and removing nodes containing
//! [`crate::state::market_seat::MarketSeat`] payloads.

use crate::state::{
    linked_list::{
        LinkedList,
        LinkedListOperations,
    },
    market_header::MarketHeader,
    sector::SectorIndex,
};

pub struct Seats;

pub type SeatsLinkedList<'a> = LinkedList<'a, Seats>;

/// Operations for the sorted, doubly linked list of nodes containing
/// [`crate::state::market_seat::MarketSeat`] payloads.
impl LinkedListOperations for Seats {
    #[inline(always)]
    fn head(header: &MarketHeader) -> SectorIndex {
        header.seat_dll_head()
    }

    #[inline(always)]
    fn set_head(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_seat_dll_head(new_index);
    }

    #[inline(always)]
    fn tail(header: &MarketHeader) -> SectorIndex {
        header.seat_dll_tail()
    }

    #[inline(always)]
    fn set_tail(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_seat_dll_tail(new_index);
    }

    #[inline(always)]
    fn increment_num_nodes(header: &mut MarketHeader) {
        header.increment_num_seats();
    }

    #[inline(always)]
    fn decrement_num_nodes(header: &mut MarketHeader) {
        header.decrement_num_seats();
    }
}
