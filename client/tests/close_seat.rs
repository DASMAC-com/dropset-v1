use client::mollusk_helpers::{
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use solana_address::Address;

#[test]
fn close_seat() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.quote.create_ata_idempotent(&user, &user),
        ])
        .program_result
        .is_ok());

    // Mint 1 base and create the seat via create_seat (which deposits 1 base).
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.mint_to_owner(&user, 1)?,
            market_ctx.create_seat(user),
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.header.num_seats, 1);

    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat");

    assert_eq!(seat.base_available, 1);
    assert_eq!(seat.quote_available, 0);
    assert_eq!(seat.user, user);

    // Close the seat. This returns the 1 base of collateral back to the user's ATA.
    assert!(mollusk
        .process_instruction_chain(&[market_ctx.close_seat(user, seat.index)])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.header.num_seats, 0);
    assert_eq!(
        mollusk.get_token_balance(&user, &market_ctx.base.mint_address),
        1
    );

    Ok(())
}
