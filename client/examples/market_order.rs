use std::collections::HashSet;

use client::{
    context::market::MarketContext,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID.into()]),
        }),
    );
    let payer = rpc.fund_new_account().await?;

    let market_ctx = MarketContext::new_market(rpc).await?;
    let register = market_ctx.register_market(payer.pubkey(), 10);

    market_ctx.base.create_ata_for(rpc, &payer).await?;
    market_ctx.quote.create_ata_for(rpc, &payer).await?;

    market_ctx.base.mint_to(rpc, &payer, 1_000_000_000).await?;
    market_ctx.quote.mint_to(rpc, &payer, 1_000_000_000).await?;

    let deposit = market_ctx.deposit_base(payer.pubkey(), 1_000_000_000, NIL);

    rpc.send_and_confirm_txn(&payer, &[&payer], &[register.into(), deposit.into()])
        .await?;

    let market = market_ctx.view_market(rpc)?;
    println!("Market after maker deposit\n{:#?}", market);

    let market_maker_seat = market_ctx
        .find_seat(rpc, &payer.pubkey())?
        .expect("Maker should have been registered on deposit");

    let (price_mantissa, base_scalar, base_exponent, quote_exponent) = (
        11_000_000,
        5,
        to_biased_exponent!(8),
        to_biased_exponent!(0),
    );

    // Post an ask so the maker user puts up quote as collateral with base to get filled.
    let is_bid = false;
    let post_ask = market_ctx.post_order(
        payer.pubkey(),
        PostOrderInstructionData::new(
            price_mantissa,
            base_scalar,
            base_exponent,
            quote_exponent,
            is_bid,
            market_maker_seat.index,
        ),
    );

    let res = rpc
        .send_and_confirm_txn(&payer, &[&payer], &[post_ask.into()])
        .await?;

    println!(
        "Post ask transaction signature: {}",
        res.parsed_transaction.signature
    );

    let market = market_ctx.view_market(rpc)?;
    println!("Market after posting maker ask:\n{:#?}", market);

    let market_maker_seat = market_ctx.find_seat(rpc, &payer.pubkey())?.unwrap();
    println!("Market maker seat after posting ask: {market_maker_seat:#?}");

    let base_before_fill = market_ctx.base.get_balance_for(rpc, &payer.pubkey())?;
    let quote_before_fill = market_ctx.quote.get_balance_for(rpc, &payer.pubkey())?;

    const ORDER_SIZE_1: u64 = 500000000 / 10;
    let market_buy = market_ctx.market_order(
        payer.pubkey(),
        MarketOrderInstructionData::new(ORDER_SIZE_1, true, true),
    );

    let market_buy_res = rpc
        .send_and_confirm_txn(&payer, &[&payer], &[market_buy.into()])
        .await?;

    println!(
        "Market buy transaction signature: {}",
        market_buy_res.parsed_transaction.signature
    );

    let market = market_ctx.view_market(rpc)?;
    println!("Market after market buy:\n{:#?}", market);

    let user_seat = market_ctx.find_seat(rpc, &payer.pubkey())?.unwrap();
    println!("Market maker seat after market buy: {user_seat:#?}");

    let base_after_fill_1 = market_ctx.base.get_balance_for(rpc, &payer.pubkey())?;
    let quote_after_fill_1 = market_ctx.quote.get_balance_for(rpc, &payer.pubkey())?;

    // ---------------------------------------------------------------------------------------------
    // Market buy again but denominate in quote with the same functional order size and ensure all
    // the amounts are the same.
    // ---------------------------------------------------------------------------------------------

    const ORDER_SIZE_2: u64 = 5500000;
    let market_buy_denom_in_quote = market_ctx.market_order(
        payer.pubkey(),
        MarketOrderInstructionData::new(ORDER_SIZE_2, true, false),
    );

    let market_buy_res_2 = rpc
        .send_and_confirm_txn(&payer, &[&payer], &[market_buy_denom_in_quote.into()])
        .await?;

    println!(
        "Market buy in quote (2) transaction signature: {}",
        market_buy_res_2.parsed_transaction.signature
    );

    let market = market_ctx.view_market(rpc)?;
    println!("Market after market buy in quote(2):\n{:#?}", market);

    let market_maker_seat = market_ctx.find_seat(rpc, &payer.pubkey())?.unwrap();
    println!("Market maker seat after market buy in quote (2): {market_maker_seat:#?}");

    let base_after_fill_2 = market_ctx.base.get_balance_for(rpc, &payer.pubkey())?;
    let quote_after_fill_2 = market_ctx.quote.get_balance_for(rpc, &payer.pubkey())?;

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
    assert_eq!(order_1.order_size, ORDER_SIZE_1);
    assert!(order_1.is_base);
    assert_eq!(order_2.order_size, ORDER_SIZE_2);
    assert!(!order_2.is_base);

    // Ensure that the amount filled and order type are the same.
    assert_eq!(order_1.is_buy, order_2.is_buy);
    assert_eq!(order_1.base_filled, order_2.base_filled);
    assert_eq!(order_1.quote_filled, order_2.quote_filled);

    let base_received_from_order_1 = base_after_fill_1 - base_before_fill;
    let base_received_from_order_2 = base_after_fill_2 - base_after_fill_1;

    let quote_spent_from_order_1 = quote_before_fill - quote_after_fill_1;
    let quote_spent_from_order_2 = quote_after_fill_1 - quote_after_fill_2;

    assert_eq!(base_received_from_order_1, base_received_from_order_2);
    assert_eq!(quote_spent_from_order_1, quote_spent_from_order_2);

    Ok(())
}
