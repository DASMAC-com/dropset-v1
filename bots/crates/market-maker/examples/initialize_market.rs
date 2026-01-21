use std::collections::HashSet;

use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::state::sector::NIL;
use solana_address::Address;
use solana_sdk::signer::Signer;
use transaction_parser::views::MarketSeatView;

#[derive(Debug)]
pub struct Info {
    pub base_mint: Address,
    pub quote_mint: Address,
    pub maker_address: Address,
    pub maker_keypair: String,
    pub market: Address,
    pub maker_seat: MarketSeatView,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID]),
        }),
    );

    let maker = test_accounts::acc_FFFF();
    let maker_address = maker.pubkey();

    let e2e = E2e::new_traders_and_market(Some(rpc), [Trader::new(maker, 10000, 10000)]).await?;

    e2e.market
        .deposit_base(maker_address, 10000, NIL)
        .send_single_signer(&e2e.rpc, maker)
        .await?;

    let info = Info {
        base_mint: e2e.market.base.mint,
        quote_mint: e2e.market.quote.mint,
        maker_address: maker.pubkey(),
        maker_keypair: maker.insecure_clone().to_base58_string(),
        market: e2e.market.market,
        maker_seat: e2e
            .view_market()?
            .seats
            .iter()
            .find(|s| s.user == maker_address)
            .expect("Should find seat")
            .clone(),
    };
    println!("{info:#?}");

    Ok(())
}
