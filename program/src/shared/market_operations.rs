//! Core reusable logic for manipulating market data structures, including inserting, removing, and
//! traversing market seats.

use dropset_interface::{
    error::DropsetError,
    state::{
        market::{
            Market,
            MarketRef,
            MarketRefMut,
        },
        market_header::MarketHeader,
        market_seat::MarketSeat,
        node::Node,
        seats_dll::SeatsLinkedList,
        sector::{
            SectorIndex,
            NIL,
            SECTOR_SIZE,
        },
        transmutable::Transmutable,
    },
};
use pinocchio::pubkey::{
    pubkey_eq,
    Pubkey,
};

pub fn insert_market_seat(
    list: &mut SeatsLinkedList,
    seat: MarketSeat,
) -> Result<SectorIndex, DropsetError> {
    let (prev_index, insert_before_index) = find_insert_before_index(list, &seat.user);
    let seat_bytes = seat.as_array();

    // Return an error early if the user already exists in the seat list at the previous index.
    if prev_index != NIL {
        // Safety: `prev_index` is non-NIL and was returned by an iterator, so it must be in-bounds.
        let prev_node = unsafe { Node::from_sector_index(list.sectors, prev_index) };
        let prev_seat = prev_node.load_payload::<MarketSeat>();
        if pubkey_eq(&seat.user, &prev_seat.user) {
            return Err(DropsetError::UserAlreadyExists);
        }
    }

    if insert_before_index == list.header.seat_dll_head() {
        list.push_front(seat_bytes)
    } else if insert_before_index == NIL {
        list.push_back(seat_bytes)
    } else {
        // Safety: `index` was returned by the iterator so it must be in-bounds.
        unsafe { list.insert_before(insert_before_index, seat_bytes) }
    }
}

/// Returns the index a node should be inserted before and the `prev_index` relative to the index
/// to be inserted at as:
///
/// (prev_index, insert_before_index)
fn find_insert_before_index(list: &SeatsLinkedList, user: &Pubkey) -> (SectorIndex, SectorIndex) {
    for (index, node) in list.iter() {
        let seat = node.load_payload::<MarketSeat>();
        if user < &seat.user {
            return (node.prev(), index);
        }
    }
    // The `prev` index at the end of the list is the tail.
    (list.header.seat_dll_tail(), NIL)
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

/// Initializes a freshly created market account. This function skips checks based on the assumption
/// that the market has just been created on-chain.
///
/// This function should *only* be called atomically in the same instruction that the market account
/// is created or in tests.
pub fn initialize_market_account_data<'a>(
    zeroed_market_account_data: &'a mut [u8],
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    market_bump: u8,
) -> Result<MarketRefMut<'a>, DropsetError> {
    let account_data_len = zeroed_market_account_data.len();
    if account_data_len < MarketHeader::LEN {
        return Err(DropsetError::UnallocatedAccountData);
    }

    let sector_bytes = account_data_len - MarketHeader::LEN;

    if sector_bytes % SECTOR_SIZE != 0 {
        return Err(DropsetError::UnalignedData);
    }

    // Safety: The account's data length was verified as at least `MARKET_HEADER_SIZE`.
    let mut market = unsafe { Market::from_bytes_mut(zeroed_market_account_data) };

    // Safety: The zeroed market account data is exclusively mutably borrowed and length-verified.
    unsafe {
        // Initialize the market header.
        MarketHeader::init(
            core::ptr::addr_of_mut!(*market.header),
            market_bump,
            base_mint,
            quote_mint,
        );
    }

    // Initialize all sectors by adding them to the free stack.
    let stack = &mut market.free_stack();
    let num_sectors = sector_bytes / SECTOR_SIZE;

    // Safety
    // Both indices are in-bounds, `start` < `end`, and the caller guarantees that the
    // account was just created, meaning it's entirely zeroed out bytes.
    unsafe { stack.convert_zeroed_bytes_to_free_nodes(0, num_sectors as u32) }?;

    Ok(market)
}

#[cfg(test)]
pub mod tests {
    use dropset_interface::state::{
        market_seat::MarketSeat,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    };
    use pinocchio_pubkey::pubkey;

    use super::initialize_market_account_data;

    extern crate std;
    use std::{
        vec,
        vec::Vec,
    };

    use super::*;

    #[test]
    fn market_insert_users() {
        const N_SECTORS: usize = 10;
        let mut bytes = [0u8; MarketHeader::LEN + SECTOR_SIZE * N_SECTORS];
        let mut market = initialize_market_account_data(
            bytes.as_mut(),
            &pubkey!("11111111111111111111111111111111111111111111"),
            &pubkey!("22222222222222222222222222222222222222222222"),
            254,
        )
        .expect("Should initialize market data");

        let mut seat_list = market.seat_list();

        let [zero, one, two, three, ten, forty] = vec![
            [vec![0; 31], vec![0]].concat().try_into().unwrap(),
            [vec![0; 31], vec![1]].concat().try_into().unwrap(),
            [vec![0; 31], vec![2]].concat().try_into().unwrap(),
            [vec![0; 31], vec![3]].concat().try_into().unwrap(),
            [vec![0; 31], vec![10]].concat().try_into().unwrap(),
            [vec![0; 31], vec![40]].concat().try_into().unwrap(),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, pk)| MarketSeat::new(pk, i as u64, (i + 2) as u64))
        .collect::<Vec<MarketSeat>>()
        .try_into()
        .unwrap();

        let seats: Vec<MarketSeat> = vec![
            three.clone(),
            two.clone(),
            forty.clone(),
            zero.clone(),
            ten.clone(),
            one.clone(),
        ];

        seats.clone().into_iter().for_each(|seat| {
            assert!(insert_market_seat(&mut seat_list, seat).is_ok());
        });

        let resulting_seat_list: Vec<(SectorIndex, &MarketSeat)> = seat_list
            .iter()
            .map(|(i, node)| (i, node.load_payload::<MarketSeat>()))
            .collect();

        let expected_order = vec![zero, one, two, three, ten, forty];

        // Check lengths before zipping.
        assert_eq!(expected_order.len(), resulting_seat_list.len());

        for (expected, actual) in resulting_seat_list
            .into_iter()
            .zip(expected_order.into_iter().enumerate())
        {
            // The `actual` user pubkeys should match the `expected` order.
            let (pk_e, pk_a) = (expected.1, &actual.1);
            assert_eq!(pk_e, pk_a);
        }
    }
}
