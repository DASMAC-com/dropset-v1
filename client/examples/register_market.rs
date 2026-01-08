use client::e2e_helpers::{
    E2e,
    Trader,
};
use solana_sdk::signature::Keypair;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let trader = Keypair::new();
    let e2e = E2e::new_traders_and_market(None, [Trader::new(&trader, 0, 0)]).await?;

    println!(
        "Transaction signature: {}",
        e2e.register_market_txn.parsed_transaction.signature
    );

    Ok(())
}
