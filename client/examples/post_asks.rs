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
    instructions::PostOrderInstructionData,
    state::sector::NIL,
};
use itertools::Itertools;
use price::{
    biased_exponent,
    OrderInfoArgs,
};
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID]),
        }),
    );

    let trader = test_accounts::acc_1111();
    let e2e = E2e::new_traders_and_market(Some(rpc), [Trader::new(trader, 10000, 10000)]).await?;

    e2e.market
        .deposit_base(trader.pubkey(), 10000, NIL)
        .send_single_signer(&e2e.rpc, trader)
        .await?;

    println!("Market after user deposit\n{:#?}", e2e.view_market().await?);

    let user_seat = e2e
        .fetch_seat(&trader.pubkey())
        .await?
        .expect("User should have been registered on deposit");

    let order_info_args =
        OrderInfoArgs::new(10_000_000, 500, biased_exponent!(0), biased_exponent!(0));

    // Post an ask. The user provides base as collateral and receives quote when filled.
    let is_bid = false;
    let post_ask_res = e2e
        .market
        .post_order(
            trader.pubkey(),
            PostOrderInstructionData::new(order_info_args.clone(), is_bid, user_seat.index),
        )
        .send_single_signer(&e2e.rpc, trader)
        .await?;

    println!(
        "Post ask transaction signature: {}",
        post_ask_res.parsed_transaction.signature
    );

    println!(
        "Market after posting user ask:\n{:#?}",
        e2e.view_market().await?
    );

    let user_seat = e2e.fetch_seat(&trader.pubkey()).await?.unwrap();
    println!("User seat after posting ask: {user_seat:#?}");

    // Post an ask. The user provides base as collateral and receives quote when filled.
    let is_bid = false;

    let ask_instructions = (1..5)
        .map(|i| {
            e2e.market
                .post_order(
                    trader.pubkey(),
                    PostOrderInstructionData::new(
                        OrderInfoArgs::new(
                            order_info_args.price_mantissa + i,
                            order_info_args.base_scalar,
                            order_info_args.base_exponent_biased,
                            order_info_args.quote_exponent_biased,
                        ),
                        is_bid,
                        user_seat.index,
                    ),
                )
                .into()
        })
        .collect_vec();

    e2e.rpc
        .send_and_confirm_txn(trader, &[trader], &ask_instructions)
        .await?;

    println!(
        "Market after posting many user asks:\n{:#?}",
        e2e.view_market().await?
    );

    let user_seat = e2e.fetch_seat(&trader.pubkey()).await?.unwrap();
    println!("User seat after posting many asks: {user_seat:#?}");

    Ok(())
}
