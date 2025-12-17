use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use dropset_interface::{
    instructions::PlaceOrderInstructionData,
    state::sector::NIL,
};
use price::to_biased_exponent;
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

    rpc.send_and_confirm_txn(&payer, &[&payer], &[register.into(), deposit.into()])
        .await?;

    let market = market_ctx.view_market(rpc)?;
    println!("Market after user deposit\n{:#?}", market);

    let user_seat = market_ctx
        .find_seat(rpc, &payer.pubkey())?
        .expect("User should have been registered on deposit");

    // Place an ask so the user puts up quote as collateral with base to get filled.
    let is_bid = false;
    let place_bid = market_ctx.place_order(
        payer.pubkey(),
        PlaceOrderInstructionData::new(
            10_000_000,
            500,
            to_biased_exponent!(0),
            to_biased_exponent!(0),
            is_bid,
            user_seat.index,
        ),
    );

    let res = rpc
        .send_and_confirm_txn(&payer, &[&payer], &[place_bid.into()])
        .await?;

    println!(
        "Place ask transaction signature: {}",
        res.parsed_transaction.signature
    );

    let market = market_ctx.view_market(rpc)?;
    println!("Market after placing user ask:\n{:#?}", market);

    Ok(())
}
