//! Core logic for manipulating and traversing [`Order`]s in the [`OrdersLinkedList`].

use dropset_interface::{
    error::DropsetError,
    state::{
        linked_list::LinkedList,
        market::{
            Market,
            MarketRefMut,
        },
        market_header::MarketHeader,
        order::{
            NextSectorIndex,
            Order,
            OrdersCollection,
        },
        sector::{
            Sector,
            SectorIndex,
            NIL,
        },
    },
};

/// Insert a new user order into the orders collection.
///
/// NOTE: this function solely inserts the order into the orders collection. It doesn't update the
/// user's seat nor does it check for duplicate prices posted by the same user.
#[inline(always)]
pub fn insert_order<T: OrdersCollection>(
    next_index: NextSectorIndex,
    list: &mut LinkedList<'_, T>,
    order: Order,
) -> Result<SectorIndex, DropsetError> {
    let next = next_index.into();
    let order_bytes = order.as_bytes();
    if next == T::head(list.header) {
        list.push_front(order_bytes)
    } else if next == NIL {
        list.push_back(order_bytes)
    } else {
        // Safety: The index used here was returned by the iterator so it must be in-bounds.
        unsafe { list.insert_before(next, order_bytes) }
    }
}

/// Converts a sector index to an order given a sector index.
///
/// Caller should ensure that `validated_sector_index` points to a valid order.
///
/// # Safety
///
/// Caller guarantees `validated_sector_index` is in-bounds of `market.sectors` bytes.
#[inline(always)]
pub unsafe fn load_order_from_sector_index<H, S>(
    market: &'_ Market<H, S>,
    validated_sector_index: SectorIndex,
) -> &'_ Order
where
    H: AsRef<MarketHeader>,
    S: AsRef<[u8]>,
{
    // Safety: Caller guarantees 'validated_sector_index' is in-bounds.
    let sector =
        unsafe { Sector::from_sector_index(market.sectors.as_ref(), validated_sector_index) };
    sector.load_payload::<Order>()
}

/// Converts a sector index to a mutable order given a sector index.
///
/// Caller should ensure that `validated_sector_index` points to a valid order.
///
/// # Safety
///
/// Caller guarantees `validated_sector_index` is in-bounds of `market.sectors` bytes.
#[inline(always)]
pub unsafe fn load_mut_order_from_sector_index<'m>(
    market: &'m mut MarketRefMut<'_>,
    validated_sector_index: SectorIndex,
) -> &'m mut Order {
    // Safety: Caller guarantees 'validated_sector_index' is in-bounds.
    let sector = unsafe { Sector::from_sector_index_mut(market.sectors, validated_sector_index) };
    sector.load_payload_mut::<Order>()
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::{
        vec,
        vec::*,
    };

    use dropset_interface::state::{
        asks_dll::{
            AskOrders,
            AskOrdersLinkedList,
        },
        bids_dll::{
            BidOrders,
            BidOrdersLinkedList,
        },
        linked_list::{
            LinkedList,
            LinkedListHeaderOperations,
        },
        market::MarketRefMut,
        market_header::MarketHeader,
        order::{
            Order,
            OrdersCollection,
        },
        sector::{
            SectorIndex,
            NIL,
            SECTOR_SIZE,
        },
        transmutable::Transmutable,
    };
    use price::{
        to_order_info,
        OrderInfoArgs,
    };
    use solana_address::Address;

    use crate::shared::{
        market_operations::initialize_market_account_data,
        order_operations::insert_order,
    };

    const N_SECTORS: usize = 10;
    const MARKET_LEN: usize = MarketHeader::LEN + SECTOR_SIZE * N_SECTORS;

    /// Test utility function to insert an order and expect (unwrap) the result.
    pub fn insert_helper<T: OrdersCollection>(
        list: &mut LinkedList<'_, T>,
        order: &Order,
    ) -> SectorIndex {
        let (next_index, _hint) = T::find_new_order_next_index(list.iter(), order);
        insert_order(next_index, list, order.clone()).expect("Should insert order")
    }

    /// Test utility function to create a simple market with a fixed amount of sectors.
    fn create_simple_market(bytes: &mut [u8; MARKET_LEN]) -> MarketRefMut<'_> {
        initialize_market_account_data(
            bytes,
            &Address::from_str_const("11111111111111111111111111111111111111111111"),
            &Address::from_str_const("22222222222222222222222222222222222222222222"),
            254,
        )
        .expect("Should initialize market data")
    }

    /// Test utility function to create orders where the output encoded price is equal to the input
    /// price mantissa. See [`OrderInfoArgs::order_at_price`].
    fn create_test_order(price_mantissa: u32, user_seat: SectorIndex) -> Order {
        let order_info = to_order_info(OrderInfoArgs::order_at_price(price_mantissa))
            .expect("The unit test should pass a valid price mantissa");

        assert_eq!(order_info.encoded_price.as_u32(), price_mantissa);
        assert_ne!(user_seat, NIL);

        Order::new(order_info, user_seat)
    }

    /// Test utility function to convert asks or bids into a vec of (encoded_price, seat) pairs.
    fn to_prices_and_seats<T: OrdersCollection>(list: &LinkedList<'_, T>) -> Vec<(u32, u32)> {
        list.iter()
            .map(|(_, sector)| {
                let order = sector.load_payload::<Order>();
                (order.encoded_price().as_u32(), order.user_seat())
            })
            .collect()
    }

    /// Test utility function to convert asks or bids into a vec of encoded prices.
    fn to_prices<T: OrdersCollection>(list: &LinkedList<'_, T>) -> Vec<u32> {
        list.iter()
            .map(|(_, sector)| sector.load_payload::<Order>().encoded_price().as_u32())
            .collect()
    }

    #[test]
    fn test_simple_order_infos() {
        const ZERO: u32 = 0;
        let get_encoded_price_u32 =
            |price_mantissa| create_test_order(price_mantissa, ZERO).encoded_price();
        assert_eq!(get_encoded_price_u32(10_000_000).as_u32(), 10_000_000);
        assert_eq!(get_encoded_price_u32(10_000_001).as_u32(), 10_000_001);
        assert_eq!(get_encoded_price_u32(10_000_002).as_u32(), 10_000_002);
        assert_eq!(get_encoded_price_u32(20_000_000).as_u32(), 20_000_000);
        assert_eq!(get_encoded_price_u32(99_999_999).as_u32(), 99_999_999);
    }

    #[test]
    fn test_time_order_precedence() {
        // Orders with the same price should be sorted based on earliest inserted.
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let (low, mid, high) = (11_111_111, 22_222_222, 33_333_333);

        let orders = [
            create_test_order(mid, 2),
            create_test_order(high, 5),
            create_test_order(mid, 3),
            create_test_order(low, 1),
            create_test_order(mid, 4),
        ];

        let asks = &mut market.asks();
        for order in orders.clone() {
            insert_helper(asks, &order);
        }

        assert_eq!(
            to_prices_and_seats(asks),
            vec![(low, 1), (mid, 2), (mid, 3), (mid, 4), (high, 5)]
        );

        let bids = &mut market.bids();
        for order in orders {
            insert_helper(bids, &order);
        }

        assert_eq!(
            to_prices_and_seats(bids),
            vec![(high, 5), (mid, 2), (mid, 3), (mid, 4), (low, 1)]
        );
    }

    #[test]
    fn test_price_order_precedence() {
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let [order_1, order_2, order_3] = [
            create_test_order(10_000_000, 1),
            create_test_order(20_000_000, 2),
            // A user can have multiple orders, so use user_seat 1 again to ensure the user seat is
            // not factored into the sorting implementation.
            create_test_order(30_000_000, 1),
        ];

        let asks = &mut market.asks();
        // Insert out of order (in terms of price) as (2, 1, 3).
        insert_helper(asks, &order_2);
        insert_helper(asks, &order_1);
        insert_helper(asks, &order_3);

        // Asks should have lowest prices first, so they should now be: (1, 2, 3).
        let expected_ask_prices_and_seats = vec![(10_000_000, 1), (20_000_000, 2), (30_000_000, 1)];
        assert_eq!(to_prices_and_seats(asks), expected_ask_prices_and_seats);

        let bids = &mut market.bids();
        // Insert out of order (in terms of price) as (2, 1, 3).
        insert_helper(bids, &order_2);
        insert_helper(bids, &order_1);
        insert_helper(bids, &order_3);

        // Bids should have highest prices first, so they should now be: (3, 2, 1).
        let expected_bid_prices_and_seats = vec![(30_000_000, 1), (20_000_000, 2), (10_000_000, 1)];
        assert_eq!(to_prices_and_seats(bids), expected_bid_prices_and_seats);
    }

    #[test]
    fn test_insert_head_mid_and_tail_asks() {
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let [order_10, order_20, order_30, order_40] = [
            create_test_order(10_000_000, 1),
            create_test_order(20_000_000, 2),
            create_test_order(30_000_000, 3),
            create_test_order(40_000_000, 4),
        ];

        let asks = &mut market.asks();

        // First order should be the head and tail: [20]
        //                                           ^^
        assert_eq!(insert_helper(asks, &order_20), AskOrders::head(asks.header));
        assert_eq!(AskOrders::head(asks.header), AskOrders::tail(asks.header));
        assert_eq!(to_prices(asks), [20_000_000]);

        // Second order should be the head. [10, 20]
        //                                   ^^
        assert_eq!(insert_helper(asks, &order_10), AskOrders::head(asks.header));
        assert_eq!(to_prices(asks), [10_000_000, 20_000_000]);

        // Third order should be the tail. [10, 20, 40]
        //                                          ^^
        assert_eq!(insert_helper(asks, &order_40), AskOrders::tail(asks.header));
        assert_eq!(to_prices(asks), [10_000_000, 20_000_000, 40_000_000]);

        // Fourth order should be neither head nor tail: [10, 20, 30, 40]
        //                                                        ^^
        let neither_head_nor_tail = insert_helper(asks, &order_30);
        assert_ne!(AskOrders::head(asks.header), neither_head_nor_tail);
        assert_ne!(AskOrders::tail(asks.header), neither_head_nor_tail);
        assert_eq!(
            to_prices(asks),
            [10_000_000, 20_000_000, 30_000_000, 40_000_000]
        );
    }
    #[test]
    fn test_insert_head_mid_and_tail_bids() {
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let [order_10, order_20, order_30, order_40] = [
            create_test_order(10_000_000, 1),
            create_test_order(20_000_000, 2),
            create_test_order(30_000_000, 3),
            create_test_order(40_000_000, 4),
        ];

        let bids = &mut market.bids();

        // First order should be the head and tail: [20]
        //                                           ^^
        assert_eq!(insert_helper(bids, &order_20), BidOrders::head(bids.header));
        assert_eq!(BidOrders::head(bids.header), BidOrders::tail(bids.header));
        assert_eq!(to_prices(bids), [20_000_000]);

        // Second order should be the head. [40, 20]
        //                                   ^^
        assert_eq!(insert_helper(bids, &order_40), BidOrders::head(bids.header));
        assert_eq!(to_prices(bids), [40_000_000, 20_000_000]);

        // Third order should be the tail. [40, 20, 10]
        //                                          ^^
        assert_eq!(insert_helper(bids, &order_10), BidOrders::tail(bids.header));
        assert_eq!(to_prices(bids), [40_000_000, 20_000_000, 10_000_000]);

        // Fourth order should be neither head nor tail: [40, 30, 20, 10]
        //                                                    ^^
        let neither_head_nor_tail = insert_helper(bids, &order_30);
        assert_ne!(BidOrders::head(bids.header), neither_head_nor_tail);
        assert_ne!(BidOrders::tail(bids.header), neither_head_nor_tail);
        assert_eq!(
            to_prices(bids),
            [40_000_000, 30_000_000, 20_000_000, 10_000_000]
        );
    }

    #[test]
    fn test_post_only_crossing_check_asks() {
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let [order_1, order_2, order_3] = [
            create_test_order(10_000_000, 1),
            create_test_order(20_000_000, 2),
            create_test_order(30_000_000, 3),
        ];

        // Placing an ask when there are no bids should succeed regardless of price.
        assert_eq!(market.bids().iter().count(), 0);
        assert!(AskOrders::post_only_crossing_check(&order_1, &market).is_ok());
        assert!(AskOrders::post_only_crossing_check(&order_2, &market).is_ok());
        assert!(AskOrders::post_only_crossing_check(&order_3, &market).is_ok());

        // Insert a single order to the bid side.
        insert_helper(&mut market.bids(), &order_2);

        let get_bids_head_price = |bids: BidOrdersLinkedList| {
            bids.iter()
                .next()
                .unwrap()
                .1
                .load_payload::<Order>()
                .encoded_price()
        };

        // Placing an ask with a higher price than the top bid should succeed.
        assert!(order_3.encoded_price() > get_bids_head_price(market.bids()));
        assert!(AskOrders::post_only_crossing_check(&order_3, &market).is_ok());

        // Placing an ask with an equal price to the top bid should fail.
        assert_eq!(order_2.encoded_price(), get_bids_head_price(market.bids()));
        assert!(AskOrders::post_only_crossing_check(&order_2, &market).is_err());

        // Placing an ask with a lower price than the top bid should fail.
        assert!(order_1.encoded_price() < get_bids_head_price(market.bids()));
        assert!(AskOrders::post_only_crossing_check(&order_1, &market).is_err());
    }

    #[test]
    fn test_post_only_crossing_check_bids() {
        let bytes = &mut [0u8; MARKET_LEN];
        let mut market = create_simple_market(bytes);

        let [order_1, order_2, order_3] = [
            create_test_order(10_000_000, 1),
            create_test_order(20_000_000, 2),
            create_test_order(30_000_000, 3),
        ];

        // Placing a bid when there are no asks should succeed regardless of price.
        assert_eq!(market.asks().iter().count(), 0);
        assert!(BidOrders::post_only_crossing_check(&order_1, &market).is_ok());
        assert!(BidOrders::post_only_crossing_check(&order_2, &market).is_ok());
        assert!(BidOrders::post_only_crossing_check(&order_3, &market).is_ok());

        // Insert a single order to the ask side.
        insert_helper(&mut market.asks(), &order_2);

        let get_asks_head_price = |asks: AskOrdersLinkedList| {
            asks.iter()
                .next()
                .unwrap()
                .1
                .load_payload::<Order>()
                .encoded_price()
        };

        // Placing a bid with a lower price than the top ask should succeed.
        assert!(order_1.encoded_price() < get_asks_head_price(market.asks()));
        assert!(BidOrders::post_only_crossing_check(&order_1, &market).is_ok());

        // Placing a bid with an equal price to the top ask should fail.
        assert_eq!(order_2.encoded_price(), get_asks_head_price(market.asks()));
        assert!(BidOrders::post_only_crossing_check(&order_2, &market).is_err());

        // Placing a bid with a higher price than the top ask should fail.
        assert!(order_3.encoded_price() > get_asks_head_price(market.asks()));
        assert!(BidOrders::post_only_crossing_check(&order_3, &market).is_err());
    }

    #[test]
    fn price_priority_checks() {
        let price_1 = create_test_order(10_000_000, 1).encoded_price();
        let price_2 = create_test_order(20_000_000, 1).encoded_price();
        let price_3 = create_test_order(30_000_000, 1).encoded_price();
        let price_4 = create_test_order(40_000_000, 1).encoded_price();

        assert!(!price_1.has_higher_bid_priority(&price_2));
        assert!(!price_2.has_higher_bid_priority(&price_3));
        assert!(!price_3.has_higher_bid_priority(&price_4));

        assert!(price_4.has_higher_bid_priority(&price_3));
        assert!(price_3.has_higher_bid_priority(&price_2));
        assert!(price_2.has_higher_bid_priority(&price_1));

        assert!(price_1.has_higher_ask_priority(&price_2));
        assert!(price_2.has_higher_ask_priority(&price_3));
        assert!(price_3.has_higher_ask_priority(&price_4));

        assert!(!price_4.has_higher_ask_priority(&price_3));
        assert!(!price_3.has_higher_ask_priority(&price_2));
        assert!(!price_2.has_higher_ask_priority(&price_1));
    }
}
