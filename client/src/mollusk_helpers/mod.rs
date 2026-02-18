use std::{
    collections::HashMap,
    path::PathBuf,
};

use dropset_interface::state::SYSTEM_PROGRAM_ID;
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
    token_instructions::create_and_initialize_token_instructions,
};

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

pub const DEFAULT_MINT_AUTHORITY: Address = pubkey!("mint1authority11111111111111111111111111111");
pub const DEFAULT_BASE_MINT: Address = pubkey!("base111111111111111111111111111111111111111");
pub const DEFAULT_QUOTE_MINT: Address = pubkey!("quote11111111111111111111111111111111111111");
pub const DEFAULT_MINT_DECIMALS: u8 = 8;
pub const DEFAULT_MARKET_ADDRESS: Address = pubkey!("7iHzqGHqCpmaEhFXbpoGnceWv7zYveUXyUdJYvYYyM1Q");
pub const DEFAULT_MARKET_BASE_ATA: Address =
    pubkey!("4n7H8mBnXnKeZh8be3u7SCFygen7pRBgF9H3NP37VtAV");
pub const DEFAULT_MARKET_QUOTE_ATA: Address =
    pubkey!("CyoUPgiQGzUB1e8SqgrMKiF5gkoezSiw4yB4x2ya5kAu");
pub const DEFAULT_TOKEN_PROGRAM: Address = SPL_TOKEN_ID;
pub const DEFAULT_NUM_SECTORS: u16 = 10;
pub const DEFAULT_MARKET_BUMP: u8 = 254;

/// Creates a default [`MarketContext`] using the default test constants.
pub fn default_market_context() -> MarketContext {
    let base = TokenContext::new(DEFAULT_BASE_MINT, DEFAULT_TOKEN_PROGRAM, DEFAULT_MINT_DECIMALS);
    let quote = TokenContext::new(DEFAULT_QUOTE_MINT, DEFAULT_TOKEN_PROGRAM, DEFAULT_MINT_DECIMALS);
    MarketContext::new(base, quote)
}

/// Creates and returns a [MolluskContext] with `dropset` and all token programs created and
/// initialized. It also creates a default market with two default tokens for base and quote.
///
/// Returns both the context and a [`MarketContext`] that can be used to build instructions for
/// the default market.
pub fn new_dropset_mollusk_context_with_default_market(
    accounts: Vec<(Address, Account)>,
) -> (MolluskContext<HashMap<Address, Account>>, MarketContext) {
    let mint_authority_addr_and_account = (
        DEFAULT_MINT_AUTHORITY,
        Account {
            data: Default::default(),
            lamports: 100_000_000_000,
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let res = new_dropset_mollusk_context(
        [accounts, [mint_authority_addr_and_account].to_vec()].concat(),
    );

    let (create_base, initialize_base) = create_and_initialize_token_instructions(
        &DEFAULT_MINT_AUTHORITY,
        &DEFAULT_BASE_MINT,
        Rent::default().minimum_balance(Mint::LEN),
        DEFAULT_MINT_DECIMALS,
        &DEFAULT_TOKEN_PROGRAM,
    )
    .expect("Should create base mint instructions");

    let (create_quote, initialize_quote) = create_and_initialize_token_instructions(
        &DEFAULT_MINT_AUTHORITY,
        &DEFAULT_QUOTE_MINT,
        Rent::default().minimum_balance(Mint::LEN),
        DEFAULT_MINT_DECIMALS,
        &DEFAULT_TOKEN_PROGRAM,
    )
    .expect("Should create quote mint instructions");

    let market = default_market_context();
    let register_market: solana_instruction::Instruction =
        market.register_market(DEFAULT_MINT_AUTHORITY, DEFAULT_NUM_SECTORS).into();

    res.process_instruction_chain(&[
        create_base,
        initialize_base,
        create_quote,
        initialize_quote,
        register_market,
    ]);

    (res, market)
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

    #[test]
    fn default_market_address() {
        let (default_market, bump) = find_market_address(&DEFAULT_BASE_MINT, &DEFAULT_QUOTE_MINT);
        let default_market_base_ata =
            get_associated_token_address(&default_market, &DEFAULT_BASE_MINT);
        let default_market_quote_ata =
            get_associated_token_address(&default_market, &DEFAULT_QUOTE_MINT);

        assert_eq!(DEFAULT_MARKET_ADDRESS, default_market);
        assert_eq!(DEFAULT_MARKET_BASE_ATA, default_market_base_ata);
        assert_eq!(DEFAULT_MARKET_QUOTE_ATA, default_market_quote_ata);
        assert_eq!(DEFAULT_MARKET_BUMP, bump);
    }

    #[test]
    fn mollusk_with_default_market() -> anyhow::Result<()> {
        let (ctx, _market) = new_dropset_mollusk_context_with_default_market(vec![]);

        let account_store = ctx.account_store.borrow();
        let default_market = account_store
            .get(&DEFAULT_MARKET_ADDRESS)
            .ok_or(anyhow!("Couldn't get default market address"))?;

        assert_eq!(default_market.owner, dropset::ID);
        assert!(!default_market.executable);
        assert_eq!(default_market.rent_epoch, 0);
        let market: MarketViewAll =
            try_market_view_all_from_owner_and_data(default_market.owner, &default_market.data)?;

        assert_eq!(market.asks.len(), 0);
        assert_eq!(market.bids.len(), 0);
        assert_eq!(market.users.len(), 0);
        assert_eq!(market.seats.len(), 0);
        assert_eq!(
            market.header,
            MarketHeaderView {
                discriminant: MARKET_ACCOUNT_DISCRIMINANT,
                num_seats: 0,
                num_bids: 0,
                num_asks: 0,
                num_free_sectors: DEFAULT_NUM_SECTORS as u32,
                free_stack_top: 0,
                seats_dll_head: NIL,
                seats_dll_tail: NIL,
                bids_dll_head: NIL,
                bids_dll_tail: NIL,
                asks_dll_head: NIL,
                asks_dll_tail: NIL,
                base_mint: DEFAULT_BASE_MINT,
                quote_mint: DEFAULT_QUOTE_MINT,
                market_bump: DEFAULT_MARKET_BUMP,
                nonce: 1, // The register market event.
                _padding: [0, 0, 0],
            }
        );

        Ok(())
    }
}
