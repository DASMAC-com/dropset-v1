use dropset_interface::{
    error::DropsetError,
    pack::Pack,
    state::{
        linked_list::LinkedList,
        market::{Market, MarketRef, MarketRefMut},
        market_header::{MarketHeader, MARKET_HEADER_SIZE},
        market_seat::MarketSeat,
        node::Node,
        sector::{NonNilSectorIndex, SectorIndex, NIL, SECTOR_SIZE},
    },
};
use pinocchio::{
    account_info::AccountInfo,
    pubkey::{pubkey_eq, Pubkey},
    ProgramResult,
};

use crate::validation::market_account_info::MarketAccountInfo;

pub fn insert_market_seat(
    list: &mut LinkedList,
    seat: MarketSeat,
) -> Result<NonNilSectorIndex, DropsetError> {
    let insert_index = find_insert_index(list, &seat.user);
    // Safety: MarketSeat adheres to all layout, alignment, and size constraints.
    let seat_bytes = unsafe { seat.as_bytes() };

    match insert_index {
        SectorIndex(0) => list.push_front(seat_bytes),
        NIL => list.push_back(seat_bytes),
        i => list.insert_before(NonNilSectorIndex::new_unchecked(i), seat_bytes),
    }
}

/// Returns the index a node should be inserted before.
///
/// ### NOTE: This function does not check for duplicates.
/// This function does not check for the user already being registered in the seat
/// list. This *will* insert duplicates without prior checks!
///
/// - `0` => Insert at the front of the list
/// - `1..n` => Insert at `n - 1`, where `n` is an in-bounds index
/// - `NIL` => Insert at the end of the list
fn find_insert_index(list: &LinkedList, user: &Pubkey) -> SectorIndex {
    for (index, node) in list.iter() {
        let seat = node.load_payload::<MarketSeat>();
        // A user that already exists in the seat list should never be passed.
        debug_assert_ne!(user, &seat.user);
        if user < &seat.user {
            // At 0, this inserts at front.
            return index.get();
        }
    }
    // Insert at back.
    NIL
}

/// Find a market seat given an index hint.
///
/// Returns an Err if the hint provided is invalid.
pub fn find_seat_with_hint<'a>(
    market: MarketRef<'a>,
    hint: NonNilSectorIndex,
    user: &Pubkey,
) -> Result<&'a Node, DropsetError> {
    let node = Node::from_non_nil_sector_index(market.sectors, hint)?;
    let seat = node.load_payload::<MarketSeat>();
    if pubkey_eq(user, &seat.user) {
        Ok(node)
    } else {
        Err(DropsetError::InvalidIndexHint)
    }
}

pub fn initialize_market_account_data<'a>(
    // This data should only have been initialized with zeroes, nothing else.
    zeroed_market_account_data: &'a mut [u8],
    base_mint: &Pubkey,
    quote_mint: &Pubkey,
    market_bump: u8,
) -> Result<MarketRefMut<'a>, DropsetError> {
    let account_data_len = zeroed_market_account_data.len();
    if account_data_len < MARKET_HEADER_SIZE {
        return Err(DropsetError::UnallocatedAccountData);
    }

    let sector_bytes = account_data_len - MARKET_HEADER_SIZE;

    if sector_bytes % SECTOR_SIZE != 0 {
        return Err(DropsetError::UnalignedData);
    }

    // Initialize the market header.
    let mut market = Market::from_bytes_mut_unchecked(zeroed_market_account_data)?;
    *market.header = MarketHeader::init(market_bump, base_mint, quote_mint);

    // Initialize all sectors by adding them to the free stack.
    let stack = &mut market.free_stack();
    let num_sectors = sector_bytes / SECTOR_SIZE;
    unsafe { stack.push_free_nodes(0, num_sectors as u32) }?;

    Ok(market)
}

#[cfg(test)]
pub mod tests {
    use super::initialize_market_account_data;
    use dropset_interface::state::{
        market_header::MARKET_HEADER_SIZE,
        market_seat::MarketSeat,
        sector::{NonNilSectorIndex, SECTOR_SIZE},
    };
    use pinocchio_pubkey::pubkey;

    extern crate std;
    use std::vec;
    use std::vec::Vec;

    use super::*;

    #[test]
    fn market_insert_users() {
        const N_SECTORS: usize = 10;
        let mut bytes = [0u8; MARKET_HEADER_SIZE + SECTOR_SIZE * N_SECTORS];
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

        let resulting_seat_list: Vec<(NonNilSectorIndex, &MarketSeat)> = seat_list
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
