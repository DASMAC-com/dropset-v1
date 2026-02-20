use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    single_signer_instruction::SingleSignerInstruction,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let payer_1 = test_accounts::acc_6666();
    let payer_2 = test_accounts::acc_7777();

    assert!(payer_1.pubkey().to_string().starts_with("6666"));
    assert!(payer_2.pubkey().to_string().starts_with("7777"));

    let traders = [
        Trader::new(payer_1, 10000, 10000),
        Trader::new(payer_2, 10000, 10000),
    ];
    let e2e = E2e::new_traders_and_market(None, traders).await?;

    // Deposit to both payers' accounts, but ensure that payer 2's seat is created
    // before payer 1 so that they're inserted out of order.
    e2e.market
        .deposit_base(payer_2.pubkey(), 1000, NIL)
        .send_single_signer(&e2e.rpc, payer_2)
        .await?;
    e2e.market
        .deposit_base(payer_1.pubkey(), 1000, NIL)
        .send_single_signer(&e2e.rpc, payer_1)
        .await?;

    let market = e2e.view_market().await?;

    // Sanity check.
    assert!(payer_1.pubkey() != payer_2.pubkey());

    // Ensure they're sorted. Payer 1 should be first despite being inserted second.
    assert_eq!(market.seats[0].user, payer_1.pubkey());
    // Payer 2 should be second.
    assert_eq!(market.seats[1].user, payer_2.pubkey());

    Ok(())
}
