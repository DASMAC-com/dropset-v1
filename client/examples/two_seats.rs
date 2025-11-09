use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};

pub const USER_111_KEY: &str =
    "65ZPkM5c2CuLcvozaVw5CRgKs9C8yHSociK85kUezr7oFCfhsK4CsFXGznEbvtn51NWdx6M33Q4o4fMBT8px6mDQ";
pub const USER_222_KEY: &str =
    "wuDnL8tvfZdoxUS3fSyuQ9CLrYjuGAAef1FYVYJumeBXnspD3193PWUVubSgB3nNo9LUbv3MzcdeGTykkq6RKBV";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::default();

    let payer_1 = Keypair::from_base58_string(USER_111_KEY);
    let payer_2 = Keypair::from_base58_string(USER_222_KEY);

    rpc.fund_account(&payer_1.pubkey()).await?;
    rpc.fund_account(&payer_2.pubkey()).await?;

    let market_ctx = MarketContext::new_market(rpc).await?;
    let register = market_ctx.register_market(payer_1.pubkey(), 10);

    market_ctx.base.create_ata_for(rpc, &payer_1).await?;
    market_ctx.quote.create_ata_for(rpc, &payer_1).await?;

    market_ctx.base.mint_to(rpc, &payer_1, 10000).await?;
    market_ctx.quote.mint_to(rpc, &payer_1, 10000).await?;

    market_ctx.base.create_ata_for(rpc, &payer_2).await?;
    market_ctx.quote.create_ata_for(rpc, &payer_2).await?;

    market_ctx.base.mint_to(rpc, &payer_2, 10000).await?;
    market_ctx.quote.mint_to(rpc, &payer_2, 10000).await?;

    let deposit_1 = market_ctx.deposit_base(payer_1.pubkey(), 1000, NIL);
    let deposit_2 = market_ctx.deposit_base(payer_2.pubkey(), 1000, NIL);

    rpc.send_and_confirm_txn(
        &payer_1,
        &[&payer_1, &payer_2],
        &[register, deposit_1, deposit_2],
    )
    .await?;

    let market = market_ctx.view_market(rpc)?;
    println!("{:#?}", market);

    Ok(())
}
