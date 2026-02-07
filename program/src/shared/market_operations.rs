//! General operations on market account data.

use dropset_interface::{
    error::DropsetError,
    state::{
        market::{
            Market,
            MarketRefMut,
        },
        market_header::MarketHeader,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    },
};
use solana_address::Address;

/// Initializes a freshly created market account. This function skips checks based on the assumption
/// that the market has just been created on-chain.
///
/// This function should *only* be called atomically in the same instruction that the market account
/// is created or in tests.
pub fn initialize_market_account_data<'a>(
    zeroed_market_account_data: &'a mut [u8],
    base_mint: &Address,
    quote_mint: &Address,
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
    unsafe { stack.convert_zeroed_bytes_to_free_sectors(0, num_sectors as u32) }?;

    Ok(market)
}

#[cfg(test)]
pub mod tests {
    use dropset_interface::state::{
        market_seat::MarketSeat,
        sector::{
            SectorIndex,
            SECTOR_SIZE,
        },
        transmutable::Transmutable,
    };
    use solana_address::Address;

    use super::initialize_market_account_data;
    use crate::shared::seat_operations::try_insert_market_seat;

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
            &Address::from_str_const("11111111111111111111111111111111111111111111"),
            &Address::from_str_const("22222222222222222222222222222222222222222222"),
            254,
        )
        .expect("Should initialize market data");

        let mut seat_list = market.seats();

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
            assert!(try_insert_market_seat(&mut seat_list, seat).is_ok());
        });

        let resulting_seat_list: Vec<(SectorIndex, &MarketSeat)> = seat_list
            .iter()
            .map(|(i, sector)| (i, sector.load_payload::<MarketSeat>()))
            .collect();

        let expected_order = vec![zero, one, two, three, ten, forty];

        // Check lengths before zipping.
        assert_eq!(expected_order.len(), resulting_seat_list.len());

        for (expected, actual) in resulting_seat_list
            .into_iter()
            .zip(expected_order.into_iter().enumerate())
        {
            // The `actual` user addresses should match the `expected` order.
            let (pk_e, pk_a) = (expected.1, &actual.1);
            assert_eq!(pk_e, pk_a);
        }
    }
}
