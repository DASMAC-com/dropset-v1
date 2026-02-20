use std::collections::HashSet;

use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    single_signer_instruction::SingleSignerInstruction,
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::NIL,
};
use price::{
    to_order_info,
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
        .deposit_base(trader.pubkey(), 1000, NIL)
        .send_single_signer(&e2e.rpc, trader)
        .await?;

    println!("Market after user deposit\n{:#?}", e2e.view_market().await?);

    let user_seat = e2e
        .fetch_seat(&trader.pubkey())
        .await?
        .expect("User should have been registered on deposit");

    let order_info_args = OrderInfoArgs::new_unscaled(10_000_000, 500);

    let order_info = to_order_info(order_info_args.clone()).expect("Should be a valid order");

    // Post an ask. The user provides base as collateral and receives quote when filled.
    let is_bid = false;
    let post_ask_res = e2e
        .market
        .post_order(
            trader.pubkey(),
            PostOrderInstructionData::new(order_info_args, is_bid, user_seat.index),
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

    let cancel_ask_res = e2e
        .market
        .cancel_order(
            user_seat.user,
            CancelOrderInstructionData::new(
                order_info.encoded_price.as_u32(),
                is_bid,
                user_seat.index,
            ),
        )
        .send_single_signer(&e2e.rpc, trader)
        .await?;

    println!(
        "Cancel ask transaction signature: {}",
        cancel_ask_res.parsed_transaction.signature
    );

    let user_seat = e2e.fetch_seat(&trader.pubkey()).await?.unwrap();
    println!("User seat after canceling ask: {user_seat:#?}");

    println!(
        "Market after canceling user ask:\n{:#?}",
        e2e.view_market().await?
    );

    Ok(())
}
