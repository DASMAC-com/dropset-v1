use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::default();

    let payer = rpc.fund_new_account().await?;

    let market_ctx = MarketContext::new_market(rpc).await?;
    let register = market_ctx.register_market(payer.pubkey(), 10);

    market_ctx.base.create_ata_for(rpc, &payer).await?;
    market_ctx.quote.create_ata_for(rpc, &payer).await?;

    market_ctx.base.mint_to(rpc, &payer, 10000).await?;
    market_ctx.quote.mint_to(rpc, &payer, 10000).await?;

    let deposit = market_ctx.deposit_base(payer.pubkey(), 1000, NIL);

    rpc.send_and_confirm_txn(&payer, &[&payer], &[register, deposit])
        .await?;

    let market = market_ctx.view_market(rpc)?;
    println!("{:#?}", market);

    let user_seat = market_ctx
        .find_seat(rpc, &payer.pubkey())?
        .expect("User should have been registered on deposit");

    let withdraw = market_ctx.withdraw_base(payer.pubkey(), 100, user_seat.index);

    rpc.send_and_confirm_txn(&payer, &[&payer], &[withdraw])
        .await?;

    Ok(())
}
