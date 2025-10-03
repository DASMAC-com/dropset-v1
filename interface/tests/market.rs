#[cfg(test)]
pub mod tests {
    use dropset_interface::state::{
        market::initialize_market,
        market_header::MARKET_HEADER_SIZE,
        market_seat::MarketSeat,
        sector::{NonNilSectorIndex, SECTOR_SIZE},
    };
    use pinocchio_pubkey::pubkey;

    #[test]
    fn market_insert_users() {
        const N_SECTORS: usize = 10;
        let mut bytes = [0u8; MARKET_HEADER_SIZE + SECTOR_SIZE * N_SECTORS];
        let mut market = initialize_market(
            bytes.as_mut(),
            202,
            &pubkey!("11111111111111111111111111111111111111111111"),
            &pubkey!("22222222222222222222222222222222222222222222"),
        )
        .expect("Should initialize market");

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
            assert!(seat_list.insert_market_seat(seat).is_ok());
        });

        let resulting_seat_list: Vec<(NonNilSectorIndex, &MarketSeat)> = seat_list
            .iter_seats()
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
