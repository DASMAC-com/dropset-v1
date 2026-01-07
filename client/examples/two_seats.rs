use client::e2e_helpers::{
    E2e,
    Trader,
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
pub const USER_AAA_PUBKEY: Pubkey =
    Pubkey::from_str_const("AAAzWLJZqUqCU3g16ztxABo4i6GBX4dr6NjfVxr7APPR");
pub const USER_BBB_PUBKEY: Pubkey =
    Pubkey::from_str_const("BBBPCe266xTrRCH6DLqfcnhGeX9QPba3K4bqM8dfaEA8");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let payer_1 = Keypair::from_base58_string(USER_AAA_KEYPAIR);
    let payer_2 = Keypair::from_base58_string(USER_BBB_KEYPAIR);

    assert_eq!(payer_1.pubkey(), USER_AAA_PUBKEY);
    assert_eq!(payer_2.pubkey(), USER_BBB_PUBKEY);

    let traders = [
        Trader::new(&payer_1, 10000, 10000),
        Trader::new(&payer_2, 10000, 10000),
    ];
    let e2e = E2e::new_traders_and_market(None, traders).await?;

    // Create payer 2's seat before payer 1 to ensure that they're inserted out of order.
    e2e.market
        .deposit_base(payer_2.pubkey(), 1000, NIL)
        .send_single_signer(&e2e.rpc, &payer_2)
        .await?;
    e2e.market
        .deposit_base(payer_1.pubkey(), 1000, NIL)
        .send_single_signer(&e2e.rpc, &payer_1)
        .await?;

    let market = e2e.view_market()?;

    // Sanity check.
    assert!(payer_1.pubkey() != payer_2.pubkey());

    // Ensure they're sorted. Payer 1 should be first despite being inserted second.
    assert_eq!(market.seats[0].user, payer_1.pubkey());
    // Payer 2 should be second.
    assert_eq!(market.seats[1].user, payer_2.pubkey());

    Ok(())
}
