use client::e2e_helpers::{
    E2e,
    Trader,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let trader = Keypair::new();
    let e2e = E2e::new_traders_and_market(None, [Trader::new(&trader, 10000, 10000)]).await?;

    e2e.send_deposit_base(&trader, 1000, NIL).await?;

    println!("{:#?}", e2e.view_market());

    let user_seat = e2e
        .find_seat(&trader.pubkey())?
        .expect("User should have been registered on deposit");

    let res = e2e
        .send_withdraw_base(&trader, 100, user_seat.index)
        .await?;

    println!(
        "Transaction signature: {}",
        res.parsed_transaction.signature
    );

    Ok(())
}
