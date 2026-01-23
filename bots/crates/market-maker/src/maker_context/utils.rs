use std::{
    collections::HashMap,
    hash::Hash,
};

use client::{
    context::market::MarketContext,
    print_kv,
};
use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::{
        order::BookSide,
        sector::SectorIndex,
    },
};
use price::{
    client_helpers::{
        decimal_pow10_i16,
        try_encoded_u32_to_decoded_decimal,
    },
    to_order_info,
};
use rust_decimal::Decimal;
use solana_address::Address;
use transaction_parser::views::{
    MarketSeatView,
    MarketViewAll,
    OrderView,
};

use crate::oanda::{
    CurrencyPair,
    OandaCandlestickResponse,
};

pub fn find_maker_seat(market: &MarketViewAll, maker: &Address) -> anyhow::Result<MarketSeatView> {
    let res = market.seats.binary_search_by_key(maker, |v| v.user);
    let seat = match res {
        Ok(found_index) => market
            .seats
            .get(found_index)
            .expect("Seat index should be valid")
            .clone(),
        Err(_insert_index) => anyhow::bail!("Couldn't find maker in seat list."),
    };

    Ok(seat)
}

/// Find an order in a collection of orders given the price, the [`BookSide`], and the seat index.
pub fn find_order<T: BookSide>(
    price: u32,
    orders: &[OrderView],
    seat_index: SectorIndex,
) -> Option<OrderView> {
    // Debug assert that the orders are in non-decreasing order.
    debug_assert!(orders
        .windows(2)
        .all(|w| T::cmp_prices(w[0].encoded_price, w[1].encoded_price).is_le()));

    let i = orders
        .binary_search_by(|o| T::cmp_prices(o.encoded_price, price))
        .ok()?;

    // `core::slice`'s binary search can return any of multiple matches.
    // Search forwards and backwards from the found index, stopping when the price changes.
    // Return when the order seat matches the passed seat index.
    //
    // This can be achieved by chaining and then searching the three iterators for the seat index:
    // 1. The first, found element: [i]
    // 2. The elements before: [i - 1, i - 2, ..] while the same `price`
    // 3. The elements after: [i + 1, i + 2, ..] while the same `price`

    let first_found = &orders[i];

    let before = orders[..i]
        .iter()
        .rev()
        .take_while(|o| o.encoded_price == price);

    let after = orders[i + 1..]
        .iter()
        .take_while(|o| o.encoded_price == price);

    std::iter::once(first_found)
        .chain(before)
        .chain(after)
        .find(|o| o.user_seat == seat_index)
        .cloned()
}

pub fn get_normalized_mid_price(
    candlestick_response: OandaCandlestickResponse,
    expected_pair: &CurrencyPair,
    market_ctx: &MarketContext,
) -> anyhow::Result<Decimal> {
    let response_pair = &candlestick_response.instrument;
    if expected_pair != response_pair {
        anyhow::bail!(
            "Maker and candlestick response pair don't match. {expected_pair} != {response_pair}"
        );
    }

    if !candlestick_response.candles.is_sorted_by_key(|c| c.time) {
        anyhow::bail!("Candlesticks aren't sorted by time (ascending).");
    }

    let latest = candlestick_response.candles.last();
    let latest_price = match latest {
        Some(candlestick) => {
            candlestick
                .mid
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("`mid` price not found in the last candlestick."))?
                .c
        }
        None => anyhow::bail!("There are zero candlesticks in the candlestick response"),
    };

    Ok(normalize_non_atoms_price(
        latest_price,
        market_ctx.base.mint_decimals,
        market_ctx.quote.mint_decimals,
    ))
}

/// Converts a token price not denominated in atoms to a token price denominated in atoms using
/// exponentiation based on the base and quote token's decimals.
pub fn normalize_non_atoms_price(
    non_atoms_price: Decimal,
    base_decimals: u8,
    quote_decimals: u8,
) -> Decimal {
    decimal_pow10_i16(
        non_atoms_price,
        quote_decimals as i16 - base_decimals as i16,
    )
}

/// Returns a pair of vectors that represents the uniquely keyed values in each hashmap.
///
/// Example in pseudo-code, where each pair represents a (k, v) pair in a hashmap.
///
/// let res = split_symmetric_difference(
///     ((1, a), (2, b), (3, c)),
///     ((3, c), (4, d), (5, e))
/// );
///
/// res == ([a, b], [d, e])
pub fn split_symmetric_difference<'a, K: Eq + Hash, V1, V2>(
    a: &'a HashMap<K, V1>,
    b: &'a HashMap<K, V2>,
) -> (Vec<&'a V1>, Vec<&'a V2>) {
    let a_uniques = a
        .iter()
        .filter(|(k, _)| !b.contains_key(k))
        .map(|(_, v)| v)
        .collect();
    let b_uniques = b
        .iter()
        .filter(|(k, _)| !a.contains_key(k))
        .map(|(_, v)| v)
        .collect();
    (a_uniques, b_uniques)
}

pub fn log_orders(
    posts: &[PostOrderInstructionData],
    cancels: &[CancelOrderInstructionData],
) -> anyhow::Result<()> {
    for cancel in cancels.iter() {
        let side = if cancel.is_bid { "bid" } else { "ask" };
        let decimal_price = try_encoded_u32_to_decoded_decimal(cancel.encoded_price)?;
        print_kv!(format!("Canceling {side} at"), format!("{decimal_price}"),);
    }

    for post in posts.iter() {
        let side = if post.is_bid { "bid" } else { "ask" };
        let encoded_price = to_order_info(
            (
                post.price_mantissa,
                post.base_scalar,
                post.base_exponent_biased,
                post.quote_exponent_biased,
            )
                .into(),
        )?
        .encoded_price;
        let decimal_price = try_encoded_u32_to_decoded_decimal(encoded_price.as_u32())?;
        print_kv!(format!("Posting {side} at"), format!("{decimal_price}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use dropset_interface::state::{
        asks_dll::AskOrders,
        bids_dll::BidOrders,
    };
    use itertools::Itertools;
    use rust_decimal::dec;
    use transaction_parser::views::OrderView;

    use super::*;

    fn create_order(price: u32, seat: u32) -> OrderView {
        OrderView {
            prev_index: 0,
            index: 0,
            next_index: 0,
            encoded_price: price,
            user_seat: seat,
            base_remaining: 0,
            quote_remaining: 0,
        }
    }

    #[test]
    fn varying_decimal_pair() {
        // Equal decimals => do nothing.
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 6, 6), dec!(1.27));

        // 10 ^ (quote - base) == 10 ^ 1 == multiply by 10
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 5, 6), dec!(12.7));

        // 10 ^ (quote - base) == 10 ^ -1 == divide by 10
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 6, 5), dec!(0.127));

        // 10 ^ (quote - base) == 10 ^ (19 - 11) == multiply by 10 ^ 8
        assert_eq!(
            normalize_non_atoms_price(dec!(1.27), 11, 19),
            dec!(127_000_000)
        );

        // 10 ^ (quote - base) == 10 ^ (11 - 19) = divide by 10 ^ 8
        assert_eq!(
            normalize_non_atoms_price(dec!(1.27), 19, 11),
            dec!(0.0000000127)
        );
    }

    #[test]
    fn find_asks_with_duplicate_prices() {
        // Note in the current order book implementation, only the price is always sorted. The seat
        // indices could be random because it's based on time priority, not seat index.
        // Asks are in ascending order because lower prices have a higher priority.
        let prices: [u32; 8] = [0, 0, 0, 1, 1, 1, 2, 3];
        let seats: [u32; 8] = [8, 9, 3, 1, 4, 7, 2, 6];

        let pairs = prices.into_iter().zip(seats).collect_vec();

        let asks = pairs
            .iter()
            .copied()
            .map(|(price, seat)| create_order(price, seat))
            .collect_vec();

        for (price, seat) in pairs {
            let order = find_order::<AskOrders>(price, &asks, seat).expect("Should find seat");
            assert_eq!(order.encoded_price, price);
            assert_eq!(order.user_seat, seat);
            assert!(find_order::<AskOrders>(price, &asks, 10000).is_none());
            assert!(find_order::<AskOrders>(10000, &asks, seat).is_none());
        }
    }

    #[test]
    fn find_bids_with_duplicate_prices() {
        // Note in the current order book implementation, only the price is always sorted. The seat
        // indices could be random because it's based on time priority, not seat index.
        // Bids are in descending order because higher prices have a higher priority.
        let prices: [u32; 8] = [3, 2, 1, 1, 1, 0, 0, 0];
        let seats: [u32; 8] = [8, 9, 3, 1, 4, 7, 2, 6];

        let pairs = prices.into_iter().zip(seats).collect_vec();

        let bids = pairs
            .iter()
            .copied()
            .map(|(price, seat)| create_order(price, seat))
            .collect_vec();

        for (price, seat) in pairs {
            let order = find_order::<BidOrders>(price, &bids, seat).expect("Should find seat");
            assert_eq!(order.encoded_price, price);
            assert_eq!(order.user_seat, seat);
            assert!(find_order::<BidOrders>(price, &bids, 10000).is_none());
            assert!(find_order::<BidOrders>(10000, &bids, seat).is_none());
        }
    }

    #[test]
    fn split_symmetric_difference_doc_example() {
        // From doc comment: a = {1: "a", 2: "b", 3: "c"}, b = {3: "c", 4: "d", 5: "e"}
        // Expected: ([a, b], [d, e])
        let a: HashMap<i32, &str> = [(1, "a"), (2, "b"), (3, "c")].into();
        let b: HashMap<i32, &str> = [(3, "c"), (4, "d"), (5, "e")].into();

        let (mut a_uniques, mut b_uniques) = split_symmetric_difference(&a, &b);
        a_uniques.sort();
        b_uniques.sort();

        assert_eq!(a_uniques, vec![&"a", &"b"]);
        assert_eq!(b_uniques, vec![&"d", &"e"]);
    }

    #[test]
    fn split_symmetric_difference_all_keys_shared() {
        // Same keys, different values returns empty vectors since filtering is by key.
        let a: HashMap<i32, &str> = [(1, "a"), (2, "b")].into();
        let b: HashMap<i32, &str> = [(1, "x"), (2, "y")].into();

        let (a_uniques, b_uniques) = split_symmetric_difference(&a, &b);

        assert!(a_uniques.is_empty());
        assert!(b_uniques.is_empty());
    }

    #[test]
    fn split_symmetric_difference_disjoint() {
        let a: HashMap<i32, &str> = [(1, "a"), (2, "b")].into();
        let b: HashMap<i32, &str> = [(3, "c"), (4, "d")].into();

        let (mut a_uniques, mut b_uniques) = split_symmetric_difference(&a, &b);
        a_uniques.sort();
        b_uniques.sort();

        assert_eq!(a_uniques, vec![&"a", &"b"]);
        assert_eq!(b_uniques, vec![&"c", &"d"]);
    }
}
