use std::collections::HashSet;

use client::{
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    single_signer_instruction::SingleSignerInstruction,
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

const MAKER_INITIAL_BASE: u64 = 10_000;
const MAKER_INITIAL_QUOTE: u64 = 10_000;

/// A helper example to bootstrap a market and a market maker. It does the following:
///
/// - Creates a market from two new tokens.
/// - Mints [`MAKER_INITIAL_BASE`] and sends it to the maker.
/// - Mints [`MAKER_INITIAL_QUOTE`] and sends it to the maker.
/// - Prints out all related info, including the generated base/quote token mint keypairs in case
///   more should be minted later.
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

    let e2e = E2e::new_traders_and_market(
        Some(rpc),
        [Trader::new(maker, MAKER_INITIAL_BASE, MAKER_INITIAL_QUOTE)],
    )
    .await?;

    e2e.market
        .deposit_base(maker_address, MAKER_INITIAL_BASE, NIL)
        .send_single_signer(&e2e.rpc, maker)
        .await?;

    let seat = e2e
        .fetch_seat(&maker_address)
        .await?
        .expect("Should have a seat")
        .index;

    e2e.market
        .deposit_quote(maker_address, MAKER_INITIAL_QUOTE, seat)
        .send_single_signer(&e2e.rpc, maker)
        .await?;

    let info = Info {
        base_mint: e2e.market.base.mint_address,
        quote_mint: e2e.market.quote.mint_address,
        maker_address: maker.pubkey(),
        maker_keypair: maker.insecure_clone().to_base58_string(),
        market: e2e.market.market,
        maker_seat: e2e
            .view_market()
            .await?
            .seats
            .iter()
            .find(|s| s.user == maker_address)
            .expect("Should find seat")
            .clone(),
    };
    println!("{info:#?}");

    Ok(())
}
