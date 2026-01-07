use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};

pub const USER_AAA_KEYPAIR: &str =
    "2zLitvKbr3wtwBNwEHgbFA9FHW3C5jn1BQkAA3whwBooGTA3PSNYNHPXiPuqXh2JJnCgCeM64aNEHxvSxnEqKMuD";
pub const USER_BBB_KEYPAIR: &str =
    "5W4uaLrJKMVuntmQ7ES9LA6RuWMZZQwVEjfHkTPHGYZ95V31yKoYTerttNZmtjgPw9U4yuYb28EC1TskmWZ2qoQp";
pub const USER_AAA_PUBKEY: &str = "AAAzWLJZqUqCU3g16ztxABo4i6GBX4dr6NjfVxr7APPR";
pub const USER_BBB_PUBKEY: &str = "BBBPCe266xTrRCH6DLqfcnhGeX9QPba3K4bqM8dfaEA8";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = &CustomRpcClient::default();

    let payer_1 = Keypair::from_base58_string(USER_AAA_KEYPAIR);
    let payer_2 = Keypair::from_base58_string(USER_BBB_KEYPAIR);

    assert_eq!(payer_1.pubkey(), Pubkey::from_str_const(USER_AAA_PUBKEY));
    assert_eq!(payer_2.pubkey(), Pubkey::from_str_const(USER_BBB_PUBKEY));

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

    rpc.send_and_confirm_txn(
        &payer_1,
        &[&payer_1, &payer_2],
        // Create payer 2's seat before payer 1 to ensure that they're inserted out of order.
        &[
            register.into(),
            market_ctx.deposit_base(payer_2.pubkey(), 1000, NIL).into(),
            market_ctx.deposit_base(payer_1.pubkey(), 1000, NIL).into(),
        ],
    )
    .await?;

    let market = market_ctx.view_market(rpc)?;
    // Sanity check.
    assert!(payer_1.pubkey() != payer_2.pubkey());

    // Ensure they're sorted. Payer 1 should be first.
    assert_eq!(
        market
            .seats
            .first()
            .expect("Should have a first element")
            .user,
        payer_1.pubkey()
    );
    // Payer 2 should be second.
    assert_eq!(
        market
            .seats
            .last()
            .expect("Should have a last element")
            .user,
        payer_2.pubkey()
    );

    Ok(())
}
