use std::collections::HashSet;

use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::{
    instructions::{
        MarketOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::NIL,
};
use itertools::Itertools;
use price::to_biased_exponent;
use solana_sdk::signer::Signer;
use transaction_parser::events::dropset_event::DropsetEvent;

fn mul_div(multiplicand: u64, multiplier: u64, divisor: u64) -> u64 {
    let res = (multiplicand as u128 * multiplier as u128) / divisor as u128;
    res as u64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID.into()]),
        }),
    );

    let maker = test_accounts::acc_1111();
    let taker = test_accounts::acc_2222();

    const MAKER_INITIAL_BASE: u64 = 1_000_000_000;
    const MAKER_INITIAL_QUOTE: u64 = 0;
    const TAKER_INITIAL_BASE: u64 = 0;
    const TAKER_INITIAL_QUOTE: u64 = 1_000_000_000;
    // The order size for both fills should be exactly the same, just denominated in different
    // assets. This example ensures that the different denominations results in the same effective
    // fill size.
    const TAKER_SIZE_IN_BASE: u64 = 50_000_000;
    const TAKER_SIZE_IN_QUOTE: u64 = 5_500_000;

    let e2e = E2e::new_traders_and_market(
        Some(rpc),
        [
            Trader::new(maker, MAKER_INITIAL_BASE, MAKER_INITIAL_QUOTE),
            Trader::new(taker, TAKER_INITIAL_BASE, TAKER_INITIAL_QUOTE),
        ],
    )
    .await?;

    e2e.check_balances(MAKER_INITIAL_BASE, MAKER_INITIAL_QUOTE, &maker.pubkey());
    e2e.check_balances(TAKER_INITIAL_BASE, TAKER_INITIAL_QUOTE, &taker.pubkey());

    e2e.market
        .deposit_base(maker.pubkey(), 1_000_000_000, NIL)
        .send_single_signer(&e2e.rpc, maker)
        .await?;

    let market_maker_seat = e2e
        .find_seat(&maker.pubkey())?
        .expect("Maker should have been registered on deposit");

    let (price_mantissa, base_scalar, base_exponent, quote_exponent) = (
        11_000_000,
        5,
        to_biased_exponent!(8),
        to_biased_exponent!(0),
    );

    // ---------------------------------------------------------------------------------------------
    // 1. Post an ask so the maker user puts up quote as collateral with base to get filled.
    // ---------------------------------------------------------------------------------------------
    let is_bid = false;
    let post_signature = e2e
        .market
        .post_order(
            maker.pubkey(),
            PostOrderInstructionData::new(
                price_mantissa,
                base_scalar,
                base_exponent,
                quote_exponent,
                is_bid,
                market_maker_seat.index,
            ),
        )
        .send_single_signer(&e2e.rpc, maker)
        .await?
        .parsed_transaction
        .signature;

    let (initial_ask_size_base, initial_ask_size_quote) = e2e
        .view_market()?
        .asks
        .first()
        .map(|v| (v.base_remaining, v.quote_remaining))
        .expect("Should have an ask");

    // Assert that the taker size ratios are equivalent to the amounts in the order to ensure that
    // both fills will be the exact same effective size.
    assert_eq!(
        TAKER_SIZE_IN_BASE as f64 / TAKER_SIZE_IN_QUOTE as f64,
        initial_ask_size_base as f64 / initial_ask_size_quote as f64
    );

    // Ensure that the taker sizes are less than the full order size.
    assert!(TAKER_SIZE_IN_BASE < initial_ask_size_base);
    assert!(TAKER_SIZE_IN_QUOTE < initial_ask_size_quote);

    println!("Post ask transaction signature: {}", post_signature);

    // Snapshot the market state and taker balances to check for expected deltas later.
    let market_before_fill = e2e.view_market()?;
    let maker_seat_before_fill = e2e.find_seat(&maker.pubkey())?.unwrap();
    let ask_before_fill = market_before_fill.asks.first().expect("Should have an ask");

    // ---------------------------------------------------------------------------------------------
    // 2. Have a taker partially fill the ask with a market buy order.
    //
    // 2a. Check the taker's base/quote balances after the fill.
    // 2b. Check the order base/quote remaining after the fill.
    // 2c. Check the maker's base/quote seat balances after the fill.
    // ---------------------------------------------------------------------------------------------
    let market_buy_res = e2e
        .market
        .market_order(
            taker.pubkey(),
            MarketOrderInstructionData::new(TAKER_SIZE_IN_BASE, true, true),
        )
        .send_single_signer(&e2e.rpc, taker)
        .await?;

    println!(
        "Market buy transaction signature: {}",
        market_buy_res.parsed_transaction.signature
    );

    let market_after_fill_1 = e2e.view_market()?;
    let maker_seat_after_fill_1 = e2e.find_seat(&maker.pubkey())?.unwrap();

    // -------------- 2a. Check the taker's base/quote balances after the first fill. --------------
    // The quote filled should be the equivalent proportional value according to the quote/base
    // remaining in the order.
    let expected_quote_filled_1 = mul_div(
        TAKER_SIZE_IN_BASE,
        initial_ask_size_quote,
        initial_ask_size_base,
    );
    // Check that the taker's received base and sent quote are correct.
    // The first order is denominated in base, so the base filled should just be the exact size.
    e2e.check_balances(
        // The taker received base.
        TAKER_INITIAL_BASE + TAKER_SIZE_IN_BASE,
        // The taker spent quote.
        TAKER_INITIAL_QUOTE - expected_quote_filled_1,
        &taker.pubkey(),
    );

    // -------------- 2b. Check the order base/quote remaining after the first fill. ---------------
    // Check that the order size properly updated the base and quote amounts.
    let ask_after_fill_1 = market_after_fill_1.asks.first().unwrap();
    // The base remaining in the order after the fill should be `initial base - order size`.
    assert_eq!(
        ask_after_fill_1.base_remaining,
        ask_before_fill.base_remaining - TAKER_SIZE_IN_BASE
    );
    // The quote remaining in the order after the fill should be:
    // `initial quote - expected quote filled`.
    assert_eq!(
        ask_after_fill_1.quote_remaining,
        ask_before_fill.quote_remaining - expected_quote_filled_1
    );

    // ------------ 2c. Check the maker's base/quote seat balances after the first fill. -----------
    // Check that the maker's seat properly updated the base and quote amounts.
    // The base shouldn't change, but the quote should increase accordingly.
    assert_eq!(
        maker_seat_after_fill_1.base_available,
        maker_seat_before_fill.base_available
    );
    // There shouldn't be any quote in the maker seat before the fill.
    assert_eq!(maker_seat_before_fill.quote_available, 0);
    // So the quote after the fill should just be the taker size in quote.
    assert_eq!(maker_seat_after_fill_1.quote_available, TAKER_SIZE_IN_QUOTE);

    // ---------------------------------------------------------------------------------------------
    // 3. Have the taker partially fill the ask again with a market buy, but denominate the order
    //    size in quote with the same function order size. Ensure all amounts are the same.
    //
    // 3a. Check the taker's base/quote balances after the second fill.
    // 3b. Check the order base/quote remaining after the second fill.
    // 3c. Check the maker's base/quote seat balances after the second fill.
    // ---------------------------------------------------------------------------------------------
    let market_buy_res_2 = e2e
        .market
        .market_order(
            taker.pubkey(),
            MarketOrderInstructionData::new(TAKER_SIZE_IN_QUOTE, true, false),
        )
        .send_single_signer(&e2e.rpc, taker)
        .await?;

    println!(
        "Market buy in quote (2) transaction signature: {}",
        market_buy_res_2.parsed_transaction.signature
    );

    let market_after_fill_2 = e2e.view_market()?;
    println!("Market after buy in quote (2):\n{:#?}", market_after_fill_2);

    let maker_seat_after_fill_2 = e2e.find_seat(&maker.pubkey())?.unwrap();
    println!("Market maker seat after buy in quote (2): {maker_seat_after_fill_2:#?}");

    // ------------- 3a. Check the taker's base/quote balances after the second fill. --------------
    // The amount filled in base/quote should be exactly as before, so simply account for the
    // expected deltas twice.
    e2e.check_balances(
        TAKER_INITIAL_BASE + (TAKER_SIZE_IN_BASE * 2),
        TAKER_INITIAL_QUOTE - (expected_quote_filled_1 * 2),
        &taker.pubkey(),
    );

    // ------------- 3b. Check the order base/quote remaining after the second fill. ---------------
    // Check that the order size updated properly again by accounting for the expected deltas twice
    // using the same logic as the balance check above.
    let ask_after_fill_2 = market_after_fill_2.asks.first().unwrap();
    assert_eq!(
        ask_after_fill_2.base_remaining,
        ask_before_fill.base_remaining - TAKER_SIZE_IN_BASE * 2
    );
    assert_eq!(
        ask_after_fill_2.quote_remaining,
        ask_before_fill.quote_remaining - expected_quote_filled_1 * 2
    );

    // ----------- 3c. Check the maker's base/quote seat balances after the second fill. -----------
    // Again, the base amount shouldn't change.
    assert_eq!(
        maker_seat_after_fill_2.base_available,
        maker_seat_before_fill.base_available
    );
    assert_eq!(
        maker_seat_after_fill_2.base_available,
        maker_seat_after_fill_1.base_available
    );
    // But the quote amount should increase.
    assert_eq!(
        maker_seat_after_fill_2.quote_available,
        maker_seat_after_fill_1.quote_available + TAKER_SIZE_IN_QUOTE
    );
    // It should also just be twice the order size in quote.
    assert_eq!(
        maker_seat_after_fill_2.quote_available,
        TAKER_SIZE_IN_QUOTE * 2
    );

    // ---------------------------------------------------------------------------------------------
    // Check that the amounts transferred according to events are the same.
    // ---------------------------------------------------------------------------------------------
    let events_1 = market_buy_res.events;
    let events_2 = market_buy_res_2.events;

    // There should be a single market order fill event for both vectors.
    let mut market_orders_1 = events_1
        .into_iter()
        .filter_map(|ev| match ev {
            DropsetEvent::MarketOrder(market_order) => Some(market_order),
            _ => None,
        })
        .collect_vec();
    let mut market_orders_2 = events_2
        .into_iter()
        .filter_map(|ev| match ev {
            DropsetEvent::MarketOrder(market_order) => Some(market_order),
            _ => None,
        })
        .collect_vec();

    assert_eq!(market_orders_1.len(), 1);
    assert_eq!(market_orders_2.len(), 1);

    let (order_1, order_2) = (
        market_orders_1.pop().unwrap(),
        market_orders_2.pop().unwrap(),
    );

    // Ensure the differences are what's expected.
    assert_eq!(order_1.order_size, TAKER_SIZE_IN_BASE);
    assert!(order_1.is_base);
    assert_eq!(order_2.order_size, TAKER_SIZE_IN_QUOTE);
    assert!(!order_2.is_base);

    // Ensure that the amount filled and order type are the same.
    assert_eq!(order_1.is_buy, order_2.is_buy);
    assert_eq!(order_1.base_filled, order_2.base_filled);
    assert_eq!(order_1.quote_filled, order_2.quote_filled);

    Ok(())
}
