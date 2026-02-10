use client::e2e_helpers::{
    no_bias_order_info_args,
    E2e,
    Trader,
};
use dropset_interface::instructions::{
    BatchReplaceInstructionData,
    Orders,
};
use price::{
    to_biased_exponent,
    OrderInfoArgs,
};
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
                Orders::new([no_bias_order_info_args(11_000_000, 1)]),
                Orders::new([
                    no_bias_order_info_args(12_000_000, 1),
                    no_bias_order_info_args(13_000_000, 2),
                    no_bias_order_info_args(14_000_000, 3),
                    no_bias_order_info_args(15_000_000, 4),
                    no_bias_order_info_args(16_000_000, 5),
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
