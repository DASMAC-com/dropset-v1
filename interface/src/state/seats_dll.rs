//! Doubly linked list utilities for traversing, inserting, and removing sectors containing
//! [`crate::state::market_seat::MarketSeat`] payloads.

use crate::state::{
    linked_list::{
        LinkedList,
        LinkedListHeaderOperations,
    },
    market_header::MarketHeader,
    sector::SectorIndex,
};

pub struct Seats;

pub type SeatsLinkedList<'a> = LinkedList<'a, Seats>;

/// Operations for the sorted, doubly linked list of sectors containing
/// [`crate::state::market_seat::MarketSeat`] payloads.
impl LinkedListHeaderOperations for Seats {
    #[inline(always)]
    fn head(header: &MarketHeader) -> SectorIndex {
        header.seats_dll_head()
    }

    #[inline(always)]
    fn set_head(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_seats_dll_head(new_index);
    }

    #[inline(always)]
    fn tail(header: &MarketHeader) -> SectorIndex {
        header.seats_dll_tail()
    }

    #[inline(always)]
    fn set_tail(header: &mut MarketHeader, new_index: SectorIndex) {
        header.set_seats_dll_tail(new_index);
    }

    #[inline(always)]
    fn increment_num_sectors(header: &mut MarketHeader) {
        header.increment_num_seats();
    }

    #[inline(always)]
    fn decrement_num_sectors(header: &mut MarketHeader) {
        header.decrement_num_seats();
    }
}
