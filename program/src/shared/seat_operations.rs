//! Core logic for manipulating and traversing [`MarketSeat`]s.

use dropset_interface::{
    error::DropsetError,
    state::{
        market::{
            Market,
            MarketRefMut,
        },
        market_header::MarketHeader,
        market_seat::MarketSeat,
        seats_dll::SeatsLinkedList,
        sector::{
            Sector,
            SectorIndex,
            NIL,
        },
    },
};
use solana_address::{
    address_eq,
    Address,
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
        let prev_sector = unsafe { Sector::from_sector_index(list.sectors, prev_index) };
        let prev_seat = prev_sector.load_payload::<MarketSeat>();
        if address_eq(&seat.user, &prev_seat.user) {
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

/// This function returns the new prev and next indices for the new sector. Thus the list would be
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
    user: &Address,
) -> (SectorIndex, SectorIndex) {
    for (index, sector) in list.iter() {
        let seat = sector.load_payload::<MarketSeat>();
        if user < &seat.user {
            return (sector.prev(), index);
        }
    }
    // If the sector is to be inserted at the end of the list, the new `prev` is the current tail
    // and the new `next` is `NIL`, since the new sector is the new tail.
    (list.header.seats_dll_tail(), NIL)
}

/// Tries to find a market seat given an index hint.
///
/// # Safety
///
/// Caller guarantees `hint` is in-bounds of `market.sectors` bytes.
pub unsafe fn find_seat_with_hint<'m, H, S>(
    market: &'m Market<H, S>,
    hint: SectorIndex,
    user: &Address,
) -> Result<&'m MarketSeat, DropsetError>
where
    H: AsRef<MarketHeader>,
    S: AsRef<[u8]>,
{
    // Safety: Caller guarantees `hint` is in-bounds.
    let sector = unsafe { Sector::from_sector_index(market.sectors.as_ref(), hint) };
    let seat = sector.load_payload::<MarketSeat>();
    if address_eq(user, &seat.user) {
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
pub unsafe fn find_mut_seat_with_hint<'m>(
    market: &'m mut MarketRefMut<'_>,
    hint: SectorIndex,
    user: &Address,
) -> Result<&'m mut MarketSeat, DropsetError> {
    // Safety: Caller guarantees `hint` is in-bounds.
    let sector = unsafe { Sector::from_sector_index_mut(market.sectors, hint) };
    let seat = sector.load_payload_mut::<MarketSeat>();
    if address_eq(user, &seat.user) {
        Ok(seat)
    } else {
        Err(DropsetError::InvalidIndexHint)
    }
}
