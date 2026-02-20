use client::mollusk_helpers::{
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::NIL,
};
use price::{
    to_order_info,
    OrderInfoArgs,
};
use solana_address::Address;

#[test]
fn post_and_cancel() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    // Mint base tokens and create the user's ATA.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.base.mint_to_owner(&user, 10_000)?,
        ])
        .program_result
        .is_ok());

    // Deposit base and create the user's seat.
    assert!(mollusk
        .process_instruction_chain(&[market_ctx.deposit_base(user, 1_000, NIL)])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat after deposit");

    let order_info_args = OrderInfoArgs::new_unscaled(10_000_000, 500);
    let order_info = to_order_info(order_info_args.clone()).expect("Should be a valid order");
    let is_bid = false;

    // Post an ask.
    assert!(mollusk
        .process_instruction_chain(&[market_ctx.post_order(
            user,
            PostOrderInstructionData::new(order_info_args, is_bid, seat.index),
        )])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.asks.len(), 1);
    assert_eq!(market.bids.len(), 0);
    assert_eq!(market.asks[0].encoded_price, order_info.encoded_price);

    // Cancel the ask.
    assert!(mollusk
        .process_instruction_chain(&[market_ctx.cancel_order(
            user,
            CancelOrderInstructionData::new(order_info.encoded_price.as_u32(), is_bid, seat.index),
        )])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.asks.len(), 0);
    assert_eq!(market.bids.len(), 0);

    Ok(())
}
