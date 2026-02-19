use anyhow::anyhow;
use client::{
    context::{
        market::MarketContext,
        token::TokenContext,
    },
    mollusk_helpers::{
        new_dropset_mollusk_context,
        utils::create_mock_user_account,
    },
    pda::find_market_address,
};
use dropset_interface::state::{
    market_header::{
        MarketHeader,
        MARKET_ACCOUNT_DISCRIMINANT,
    },
    sector::{
        Sector,
        NIL,
    },
    transmutable::Transmutable,
};
use mollusk_svm::result::Check;
use solana_address::Address;
use solana_sdk::{
    program_pack::Pack,
    rent::Rent,
};
use spl_token_interface::state::Mint;
use transaction_parser::{
    program_ids::SPL_TOKEN_ID,
    views::{
        try_market_view_all_from_owner_and_data,
        MarketHeaderView,
        MarketViewAll,
    },
};

#[test]
fn register_market() -> anyhow::Result<()> {
    let mock_funder = create_mock_user_account(Address::new_unique(), 100_000_000_000);
    let funder = mock_funder.0;
    let mollusk = new_dropset_mollusk_context(vec![mock_funder]);
    let market_ctx = MarketContext::new(
        TokenContext::new(Some(funder), Address::new_unique(), SPL_TOKEN_ID, 8),
        TokenContext::new(Some(funder), Address::new_unique(), SPL_TOKEN_ID, 8),
    );

    // Create the tokens.
    mollusk.process_instruction_chain(
        &market_ctx
            .create_tokens(funder, Rent::default().minimum_balance(Mint::LEN))
            .expect("Should create token instructions"),
    );

    // Register the market and run checks on the account post-registration.
    let num_sectors = 23;
    let ixn_res = mollusk.process_and_validate_instruction(
        &market_ctx.register_market(funder, num_sectors as u16),
        &[Check::account(&market_ctx.market)
            .executable(false)
            .owner(&dropset::ID)
            .rent_exempt()
            .space(MarketHeader::LEN + Sector::LEN * num_sectors)
            .build()],
    );

    let market_account_data = &ixn_res
        .get_account(&market_ctx.market)
        .ok_or(anyhow!("Couldn't find market account"))?
        .data;

    // Run more in-depth checks on the state of the market account.
    let market_view: MarketViewAll =
        try_market_view_all_from_owner_and_data(dropset::ID, market_account_data)?;

    let (_, bump) = find_market_address(
        &market_ctx.base.mint_address,
        &market_ctx.quote.mint_address,
    );

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
            num_free_sectors: num_sectors as u32,
            free_stack_top: 0,
            seats_dll_head: NIL,
            seats_dll_tail: NIL,
            bids_dll_head: NIL,
            bids_dll_tail: NIL,
            asks_dll_head: NIL,
            asks_dll_tail: NIL,
            base_mint: market_ctx.base.mint_address,
            quote_mint: market_ctx.quote.mint_address,
            market_bump: bump,
            nonce: 1, // The register market event.
            _padding: [0, 0, 0],
        }
    );

    Ok(())
}
