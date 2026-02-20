use std::{
    collections::HashMap,
    path::PathBuf,
};

use mollusk_svm::{
    Mollusk,
    MolluskContext,
};
use solana_account::Account;
use solana_address::Address;
use solana_sdk::{
    program_pack::Pack,
    pubkey,
    rent::Rent,
};
use spl_token_interface::state::Mint;
use transaction_parser::program_ids::SPL_TOKEN_ID;

use crate::{
    context::{
        market::MarketContext,
        token::TokenContext,
    },
    mollusk_helpers::utils::create_mock_user_account,
};

pub mod helper_trait;
pub mod utils;

/// Converts an input deploy file to a program name used by the [`Mollusk::new`] function.
///
/// Requires the full file name; for example, `dropset.so` would return the absolute path version of
/// `../target/deploy/dropset`, which is exactly what [`Mollusk::new`] expects.
fn deploy_file_to_program_name(program_name: &str) -> String {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("target/deploy/")
        .join(program_name)
        .canonicalize()
        .map(|p| {
            p.to_str()
                .expect("Path should convert to a &str")
                .strip_suffix(".so")
                .expect("Deploy file should have an `.so` suffix")
                .to_string()
        })
        .expect("Should create relative target/deploy/ path")
}

/// Creates and returns a [`MolluskContext`] with the following created and initialized:
/// - The `dropset` program
/// - The SPL token program
/// - The SPL token 2022 program
/// - The associated token program
/// - The accounts passed
pub fn new_dropset_mollusk_context(
    accounts: Vec<(Address, Account)>,
) -> MolluskContext<HashMap<Address, Account>> {
    let mut mollusk = Mollusk::new(&dropset::ID, &deploy_file_to_program_name("dropset.so"));
    mollusk_svm_programs_token::token::add_program(&mut mollusk);
    mollusk_svm_programs_token::token2022::add_program(&mut mollusk);
    mollusk_svm_programs_token::associated_token::add_program(&mut mollusk);

    // Create mollusk context with the simple hashmap implementation for the AccountStore.
    let context = mollusk.with_context(HashMap::new());

    // Create each account passed in at its respective address using the specified account data.
    // This "funds" accounts in the sense that it will create the account with the specified
    // lamport balance in its account data.
    for (address, account) in accounts {
        context.account_store.borrow_mut().insert(address, account);
    }

    context
}

pub const MOLLUSK_DEFAULT_MINT_AUTHORITY: Address =
    pubkey!("mint1authority11111111111111111111111111111");
pub const MOLLUSK_DEFAULT_NUM_SECTORS: u16 = 10;

pub const MOLLUSK_DEFAULT_BASE_TOKEN: TokenContext = TokenContext::new(
    Some(MOLLUSK_DEFAULT_MINT_AUTHORITY),
    pubkey!("base111111111111111111111111111111111111111"),
    SPL_TOKEN_ID,
    8,
);

pub const MOLLUSK_DEFAULT_QUOTE_TOKEN: TokenContext = TokenContext::new(
    Some(MOLLUSK_DEFAULT_MINT_AUTHORITY),
    pubkey!("quote11111111111111111111111111111111111111"),
    SPL_TOKEN_ID,
    8,
);

pub const MOLLUSK_DEFAULT_MARKET: MarketContext = MarketContext {
    market: pubkey!("7iHzqGHqCpmaEhFXbpoGnceWv7zYveUXyUdJYvYYyM1Q"),
    base: MOLLUSK_DEFAULT_BASE_TOKEN,
    quote: MOLLUSK_DEFAULT_QUOTE_TOKEN,
    base_market_ata: pubkey!("4n7H8mBnXnKeZh8be3u7SCFygen7pRBgF9H3NP37VtAV"),
    quote_market_ata: pubkey!("CyoUPgiQGzUB1e8SqgrMKiF5gkoezSiw4yB4x2ya5kAu"),
};

/// Creates and returns a [MolluskContext] with `dropset` and all token programs created and
/// initialized. It also creates a default market with two default tokens for base and quote.
///
/// Returns both the context and a [`MarketContext`] that can be used to build instructions for
/// the default market.
pub fn new_dropset_mollusk_context_with_default_market(
    accounts: &[(Address, Account)],
) -> (MolluskContext<HashMap<Address, Account>>, MarketContext) {
    let mint_authority_addr_and_account =
        create_mock_user_account(MOLLUSK_DEFAULT_MINT_AUTHORITY, 100_000_000_000);

    let res = new_dropset_mollusk_context([accounts, &[mint_authority_addr_and_account]].concat());

    let create_tokens = MOLLUSK_DEFAULT_MARKET
        .create_tokens(
            MOLLUSK_DEFAULT_MINT_AUTHORITY,
            Rent::default().minimum_balance(Mint::LEN),
        )
        .expect("Should create token instructions");

    let register_market = MOLLUSK_DEFAULT_MARKET
        .register_market(MOLLUSK_DEFAULT_MINT_AUTHORITY, MOLLUSK_DEFAULT_NUM_SECTORS);

    res.process_instruction_chain(&[create_tokens, vec![register_market]].concat());

    (res, MOLLUSK_DEFAULT_MARKET)
}

#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use dropset_interface::state::{
        market_header::MARKET_ACCOUNT_DISCRIMINANT,
        sector::NIL,
    };
    use spl_associated_token_account_interface::address::get_associated_token_address;
    use transaction_parser::views::{
        try_market_view_all_from_owner_and_data,
        MarketHeaderView,
        MarketViewAll,
    };

    use super::*;
    use crate::pda::find_market_address;

    #[test]
    fn dropset_program_path() {
        let dropset = deploy_file_to_program_name("dropset.so");
        assert!(dropset.ends_with("dropset"));

        // Ensure the program deploy path is a valid file.
        assert!(PathBuf::from([dropset.as_str(), ".so"].concat()).is_file());
    }

    /// Verifies that the hardcoded addresses in [`MOLLUSK_DEFAULT_MARKET`] match the PDA
    /// derivation from [`MarketContext::new`].
    #[test]
    fn default_market_const_matches_derived() {
        let derived = MarketContext::new(MOLLUSK_DEFAULT_BASE_TOKEN, MOLLUSK_DEFAULT_QUOTE_TOKEN);
        assert_eq!(MOLLUSK_DEFAULT_MARKET.market, derived.market);
        assert_eq!(
            MOLLUSK_DEFAULT_MARKET.base_market_ata,
            derived.base_market_ata
        );
        assert_eq!(
            MOLLUSK_DEFAULT_MARKET.quote_market_ata,
            derived.quote_market_ata
        );
        assert_eq!(
            MOLLUSK_DEFAULT_MARKET.base_market_ata,
            get_associated_token_address(&derived.market, &MOLLUSK_DEFAULT_BASE_TOKEN.mint_address)
        );
        assert_eq!(
            MOLLUSK_DEFAULT_MARKET.quote_market_ata,
            get_associated_token_address(
                &derived.market,
                &MOLLUSK_DEFAULT_QUOTE_TOKEN.mint_address
            )
        );
    }

    #[test]
    fn mollusk_with_default_market() -> anyhow::Result<()> {
        let (derived_market, bump) = find_market_address(
            &MOLLUSK_DEFAULT_BASE_TOKEN.mint_address,
            &MOLLUSK_DEFAULT_QUOTE_TOKEN.mint_address,
        );
        let (ctx, market) = new_dropset_mollusk_context_with_default_market(&[]);
        assert_eq!(market.market, derived_market);

        let account_store = ctx.account_store.borrow();
        let market_account = account_store
            .get(&market.market)
            .ok_or(anyhow!("Couldn't get default market address"))?;

        assert_eq!(market_account.owner, dropset::ID);
        assert!(!market_account.executable);
        assert_eq!(market_account.rent_epoch, 0);
        let market_view: MarketViewAll =
            try_market_view_all_from_owner_and_data(market_account.owner, &market_account.data)?;

        assert_eq!(market_view.asks.len(), 0);
        assert_eq!(market_view.bids.len(), 0);
        assert_eq!(market_view.users.len(), 0);
        assert_eq!(market_view.seats.len(), 0);
        assert_eq!(
            market_view.header,
            MarketHeaderView {
                discriminant: MARKET_ACCOUNT_DISCRIMINANT,
                num_seats: 0,
                num_bids: 0,
                num_asks: 0,
                num_free_sectors: MOLLUSK_DEFAULT_NUM_SECTORS as u32,
                free_stack_top: 0,
                seats_dll_head: NIL,
                seats_dll_tail: NIL,
                bids_dll_head: NIL,
                bids_dll_tail: NIL,
                asks_dll_head: NIL,
                asks_dll_tail: NIL,
                base_mint: MOLLUSK_DEFAULT_BASE_TOKEN.mint_address,
                quote_mint: MOLLUSK_DEFAULT_QUOTE_TOKEN.mint_address,
                market_bump: bump,
                nonce: 1, // The register market event.
                _padding: [0, 0, 0],
            }
        );

        Ok(())
    }
}
