use std::collections::HashSet;

use client::{
    context::market::MarketContext,
    test_accounts::*,
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::state::sector::SectorIndex;
use solana_instruction::Instruction;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID.into()]),
        }),
    );

    let payer = rpc.fund_new_account().await?;

    let market_ctx = MarketContext::new_market(rpc).await?;

    rpc.send_and_confirm_txn(
        &payer,
        &[&payer],
        &[market_ctx.register_market(payer.pubkey(), 10)],
    )
    .await?;

    let signers: Vec<&Keypair> = vec![&USER_1, &USER_2, &USER_3, &USER_4, &USER_5];

    for user in signers.iter() {
        rpc.fund_account(&user.pubkey()).await?;
        market_ctx.base.create_ata_for(rpc, user).await?;
        market_ctx.quote.create_ata_for(rpc, user).await?;
        market_ctx.base.mint_to(rpc, user, 10000).await?;
        market_ctx.quote.mint_to(rpc, user, 10000).await?;
    }

    let user_pks: Vec<Pubkey> = signers.iter().map(|u| u.pubkey()).collect();

    let seat_creations: Vec<Instruction> = user_pks
        .iter()
        .map(|pk| market_ctx.create_seat(*pk))
        .collect();

    rpc.send_and_confirm_txn(signers[0], &signers, &seat_creations)
        .await?;

    let seats: Vec<SectorIndex> = user_pks
        .iter()
        .map(|user| {
            market_ctx
                .find_seat(rpc, user)
                .ok()
                .flatten()
                .expect("User should have a seat")
                .index
        })
        .collect();

    let deposits_and_withdraws: Vec<Instruction> = user_pks
        .iter()
        .zip(seats)
        .flat_map(|(user, seat)| {
            [
                market_ctx.deposit_base(*user, 100, seat),
                market_ctx.withdraw_base(*user, 50, seat),
            ]
        })
        .collect();

    rpc.send_and_confirm_txn(signers[0], &signers, &deposits_and_withdraws)
        .await?;

    Ok(())
}
