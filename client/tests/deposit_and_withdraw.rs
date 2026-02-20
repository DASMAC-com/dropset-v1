use client::mollusk_helpers::{
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use dropset_interface::state::sector::NIL;
use solana_address::Address;
use transaction_parser::views::MarketSeatView;

#[test]
fn deposit_and_withdraw() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    let initial_base: u64 = 10_000;
    let initial_quote: u64 = 20_000;

    // Create the user ATAs for base and quote and mint to them the initial amounts.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.quote.create_ata_idempotent(&user, &user),
            market_ctx.base.mint_to_owner(&user, initial_base)?,
            market_ctx.quote.mint_to_owner(&user, initial_quote)?,
        ])
        .program_result
        .is_ok());

    let base_balance = mollusk.get_token_balance(&user, &market_ctx.base.mint_address);
    let quote_balance = mollusk.get_token_balance(&user, &market_ctx.quote.mint_address);
    assert_eq!(base_balance, 10_000);
    assert_eq!(quote_balance, 20_000);

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.deposit_base(user, 1_000, NIL),
            market_ctx.deposit_quote(user, 1_000, 0), // The seat is the first seat on the market.
        ])
        .program_result
        .is_ok());

    let base_balance = mollusk.get_token_balance(&user, &market_ctx.base.mint_address);
    let quote_balance = mollusk.get_token_balance(&user, &market_ctx.quote.mint_address);
    assert_eq!(base_balance, 9_000);
    assert_eq!(quote_balance, 19_000);

    let market = mollusk.view_market(&market_ctx.market);

    let seat = market.seats.iter().find(|seat| seat.user == user);
    assert_eq!(
        seat,
        Some(&MarketSeatView {
            base_available: 1_000,
            quote_available: 1_000,
            prev_index: NIL,
            index: 0, // The seat is the first seat on the market.
            next_index: NIL,
            user,
            user_order_sectors: Default::default(),
        })
    );

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.withdraw_base(user, 1_000, 0),
            market_ctx.withdraw_quote(user, 1_000, 0),
        ])
        .program_result
        .is_ok());

    let base_balance = mollusk.get_token_balance(&user, &market_ctx.base.mint_address);
    let quote_balance = mollusk.get_token_balance(&user, &market_ctx.quote.mint_address);
    assert_eq!(base_balance, 10_000);
    assert_eq!(quote_balance, 20_000);

    Ok(())
}
