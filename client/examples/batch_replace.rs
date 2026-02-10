use client::e2e_helpers::{
    E2e,
    Trader,
};
use dropset_interface::instructions::{
    BatchReplaceInstructionData,
    UnvalidatedOrders,
};
use price::OrderInfoArgs;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let trader = Keypair::new();
    let e2e = E2e::new_traders_and_market(None, [Trader::new(&trader, 0, 0)]).await?;

    let res = e2e
        .market
        .batch_replace(
            trader.pubkey(),
            BatchReplaceInstructionData::new(
                0,
                UnvalidatedOrders::new([OrderInfoArgs::new_unscaled(11_000_000, 1)]),
                UnvalidatedOrders::new([
                    OrderInfoArgs::new_unscaled(12_000_000, 1),
                    OrderInfoArgs::new_unscaled(13_000_000, 2),
                    OrderInfoArgs::new_unscaled(14_000_000, 3),
                    OrderInfoArgs::new_unscaled(15_000_000, 4),
                    OrderInfoArgs::new_unscaled(16_000_000, 5),
                ]),
            ),
        )
        .send_single_signer(&e2e.rpc, &trader)
        .await?;

    for msg in res.parsed_transaction.log_messages {
        println!("{msg}");
    }

    println!(
        "Transaction signature: {}",
        e2e.register_market_txn.parsed_transaction.signature
    );

    Ok(())
}
