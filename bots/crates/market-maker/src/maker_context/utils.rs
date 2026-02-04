use std::{
    collections::HashMap,
    hash::Hash,
};

use client::{
    context::market::MarketContext,
    print_kv,
};
use dropset_interface::instructions::{
    CancelOrderInstructionData,
    PostOrderInstructionData,
};
use price::{
    client_helpers::{
        decimal_pow10_i16,
        try_encoded_u32_to_decoded_decimal,
    },
    to_order_info,
};
use rust_decimal::Decimal;

use crate::oanda::{
    CurrencyPair,
    OandaCandlestickResponse,
};

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

    let sorted_candles = {
        let mut candles = candlestick_response.candles;
        candles.sort_by_key(|c| c.time);
        candles
    };

    let latest_price = match sorted_candles.last() {
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

/// Returns values from each hashmap whose keys don't exist in the other.
///
/// Filtering is by key only; values are ignored when determining uniqueness.
///
/// For example, with hashmap inputs `a` and `b`:
///
/// a: (1, "a"), (2, "b"), (3, "c")]
/// b: (3, "x"), (4, "d"), (5, "e")]
///
/// This function would return two vecs: ["a", "b"] and ["d", "e"].
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
        let encoded_price = to_order_info(post.order_info_args.clone())?.encoded_price;
        let decimal_price = try_encoded_u32_to_decoded_decimal(encoded_price.as_u32())?;
        print_kv!(format!("Posting {side} at"), format!("{decimal_price}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

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
}
