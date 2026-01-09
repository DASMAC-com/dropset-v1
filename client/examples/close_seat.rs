use std::collections::HashSet;

use client::{
    e2e_helpers::{
        E2e,
        Trader,
    },
    print_kv,
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
    LogColor,
};
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: None,
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID.into()]),
        }),
    );

    let trader = Keypair::new();

    let e2e = E2e::new_traders_and_market(Some(rpc), [Trader::new(&trader, 10000, 10000)]).await?;

    // Create a seat for the trader.
    e2e.market
        .create_seat(trader.pubkey())
        .send_single_signer(&e2e.rpc, &trader)
        .await?;

    let market = e2e.view_market()?;
    print_kv!("Seats before", market.header.num_seats, LogColor::Info);

    let user_seat = e2e
        .find_seat(&trader.pubkey())?
        .expect("User should have been registered on deposit");

    e2e.market
        .close_seat(trader.pubkey(), user_seat.index)
        .send_single_signer(&e2e.rpc, &trader)
        .await?;

    let market = e2e.view_market()?;
    print_kv!("Seats after", market.header.num_seats, LogColor::Info);

    Ok(())
}
