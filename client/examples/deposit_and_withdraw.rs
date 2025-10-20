use client::{
    context::market::MarketContext,
    transactions::{
        fund_account,
        send_transaction,
    },
};
use dropset_interface::{
    instructions::{
        DepositInstructionData,
        RegisterMarketInstructionData,
        WithdrawInstructionData,
    },
    state::sector::NIL,
};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc =
        &RpcClient::new_with_commitment("http://localhost:8899", CommitmentConfig::confirmed());

    let payer = fund_account(rpc, None).await?;
    let market_ctx = MarketContext::new_market(rpc).await?;

    let register = market_ctx.register_market(
        payer.pubkey(),
        RegisterMarketInstructionData { num_sectors: 10 },
    );

    market_ctx.base.create_ata_for(rpc, &payer).await?;
    market_ctx.quote.create_ata_for(rpc, &payer).await?;

    market_ctx.base.mint_to(rpc, &payer, 10000).await?;
    market_ctx.quote.mint_to(rpc, &payer, 10000).await?;

    let deposit = market_ctx.deposit_base(
        payer.pubkey(),
        DepositInstructionData {
            amount: 1000,
            sector_index_hint: NIL,
        },
    );

    send_transaction(rpc, &payer, &[&payer], &[register, deposit]).await?;

    let market = market_ctx.view_market(rpc)?;
    println!("{:#?}", market);

    let user_seat = market_ctx
        .find_seat(rpc, &payer.pubkey())?
        .expect("User should have been registered on deposit");

    let withdraw = market_ctx.withdraw_base(
        payer.pubkey(),
        WithdrawInstructionData {
            amount: 100,
            sector_index_hint: user_seat.index,
        },
    );

    send_transaction(rpc, &payer, &[&payer], &[withdraw]).await?;

    Ok(())
}
