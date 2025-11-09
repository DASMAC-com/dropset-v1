use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::default();

    let payer = rpc.fund_new_account().await?;
    let market_ctx = MarketContext::new_market(rpc).await?;

    let register = market_ctx.register_market(payer.pubkey(), 10);

    let res = rpc
        .send_and_confirm_txn(&payer, &[&payer], &[register])
        .await?;

    println!("Transaction signature: {res}");

    Ok(())
}
