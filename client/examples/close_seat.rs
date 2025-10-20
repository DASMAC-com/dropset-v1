use client::{
    context::market::MarketContext,
    logs::log_info,
    transactions::{
        fund_account,
        send_transaction_with_config,
        SendTransactionConfig,
    },
};
use dropset_interface::{
    instructions::{
        CloseSeatInstructionData,
        DepositInstructionData,
        RegisterMarketInstructionData,
    },
    state::sector::NIL,
};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signer::Signer;

const CONFIG: Option<SendTransactionConfig> = Some(SendTransactionConfig {
    compute_budget: None,
    debug_logs: Some(true),
});

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

    send_transaction_with_config(rpc, &payer, &[&payer], &[register, deposit], CONFIG).await?;

    let market = market_ctx.view_market(rpc)?;
    log_info("Seats before", market.header.num_seats);

    let user_seat = market_ctx
        .find_seat(rpc, &payer.pubkey())?
        .expect("User should have been registered on deposit");

    let close_seat = market_ctx.close_seat(
        payer.pubkey(),
        CloseSeatInstructionData {
            sector_index_hint: user_seat.index,
        },
    );

    send_transaction_with_config(rpc, &payer, &[&payer], &[close_seat], CONFIG).await?;

    let market = market_ctx.view_market(rpc)?;
    log_info("Seats after", market.header.num_seats);

    Ok(())
}
