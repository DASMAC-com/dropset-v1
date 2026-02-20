use client::mollusk_helpers::{
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use dropset_interface::{
    instructions::PostOrderInstructionData,
    state::sector::NIL,
};
use itertools::Itertools;
use price::OrderInfoArgs;
use solana_address::Address;

#[test]
fn post_asks() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.base.mint_to_owner(&user, 100_000)?,
        ])
        .program_result
        .is_ok());

    // Deposit base and create the user's seat.
    assert!(mollusk
        .process_instruction_chain(&[market_ctx.deposit_base(user, 10_000, NIL)])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat after deposit");

    // Post 5 asks at distinct prices in ascending order (lowest price = highest ask priority).
    let is_bid = false;
    let num_asks: u32 = 5;
    let post_instructions = (1..=num_asks)
        .map(|i| {
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::new_unscaled(10_000_000 + i, 100),
                    is_bid,
                    seat.index,
                ),
            )
        })
        .collect_vec();

    assert!(mollusk
        .process_instruction_chain(&post_instructions)
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.asks.len(), num_asks as usize);
    assert_eq!(market.bids.len(), 0);

    let total_ask_collateral = market
        .asks
        .iter()
        .map(|ask| ask.base_remaining)
        .sum::<u64>();

    // Ensure that the ask collateral was subtracted from the seat's ask remaining.
    assert_eq!(
        10_000 - total_ask_collateral,
        market_ctx
            .find_seat(&market.seats, &user)
            .unwrap()
            .base_available
    );

    // Ensure that the orders were sorted upon insertion properly by checking that each order has
    // a higher price priority than the next.
    assert!(market
        .asks
        .iter()
        .tuple_windows()
        .all(|(a, b)| a.encoded_price.has_higher_ask_priority(&b.encoded_price)));

    Ok(())
}
