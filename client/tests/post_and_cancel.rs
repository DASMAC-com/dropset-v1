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
use itertools::Itertools;
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

// Using order_at_price: base_atoms = 10^15 per ask, quote_atoms = price_mantissa / 10 per bid.
// Asks use high prices (60M–99M), bids use low prices (10M–50M) to avoid crossing the book.
#[test]
fn post_and_cancel_maintains_sort_order() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.quote.create_ata_idempotent(&user, &user),
            market_ctx
                .base
                .mint_to_owner(&user, 10_000_000_000_000_000)?,
            market_ctx.quote.mint_to_owner(&user, 50_000_000)?,
        ])
        .program_result
        .is_ok());

    // Deposit base (creates seat at index 0) then quote.
    // Peak ask collateral: 5 * 10^15 base. Peak bid collateral: 15_000_000 quote.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.deposit_base(user, 6_000_000_000_000_000, NIL),
            market_ctx.deposit_quote(user, 20_000_000, 0),
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat");

    // Post 5 asks and 5 bids at known prices.
    let ask_prices: [u32; 5] = [60_000_000, 70_000_000, 80_000_000, 90_000_000, 99_000_000];
    let bid_prices: [u32; 5] = [10_000_000, 20_000_000, 30_000_000, 40_000_000, 50_000_000];

    let post_asks = ask_prices.iter().map(|&p| {
        market_ctx.post_order(
            user,
            PostOrderInstructionData::new(OrderInfoArgs::order_at_price(p), false, seat.index),
        )
    });
    let post_bids = bid_prices.iter().map(|&p| {
        market_ctx.post_order(
            user,
            PostOrderInstructionData::new(OrderInfoArgs::order_at_price(p), true, seat.index),
        )
    });

    assert!(mollusk
        .process_instruction_chain(&post_asks.chain(post_bids).collect_vec())
        .program_result
        .is_ok());

    // Cancel the 2nd and 3rd asks/bids by price, leaving gaps.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.cancel_order(
                user,
                CancelOrderInstructionData::new(70_000_000, false, seat.index)
            ),
            market_ctx.cancel_order(
                user,
                CancelOrderInstructionData::new(80_000_000, false, seat.index)
            ),
            market_ctx.cancel_order(
                user,
                CancelOrderInstructionData::new(20_000_000, true, seat.index)
            ),
            market_ctx.cancel_order(
                user,
                CancelOrderInstructionData::new(40_000_000, true, seat.index)
            ),
        ])
        .program_result
        .is_ok());

    // Fill the gaps and add one beyond the end of each book side.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(65_000_000),
                    false,
                    seat.index
                )
            ),
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(75_000_000),
                    false,
                    seat.index
                )
            ),
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(95_000_000),
                    false,
                    seat.index
                )
            ),
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(15_000_000),
                    true,
                    seat.index
                )
            ),
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(35_000_000),
                    true,
                    seat.index
                )
            ),
            market_ctx.post_order(
                user,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(45_000_000),
                    true,
                    seat.index
                )
            ),
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.asks.len(), 6);
    assert_eq!(market.bids.len(), 6);

    // Verify sort order is maintained after all the insertions and removals.
    assert!(market
        .asks
        .iter()
        .tuple_windows()
        .all(|(a, b)| a.encoded_price.has_higher_ask_priority(&b.encoded_price)));
    assert!(market
        .bids
        .iter()
        .tuple_windows()
        .all(|(a, b)| a.encoded_price.has_higher_bid_priority(&b.encoded_price)));

    // Verify exact price sequence using the order_at_price invariant (encoded_price == mantissa).
    let ask_encoded: Vec<u32> = market
        .asks
        .iter()
        .map(|o| o.encoded_price.as_u32())
        .collect();
    let bid_encoded: Vec<u32> = market
        .bids
        .iter()
        .map(|o| o.encoded_price.as_u32())
        .collect();

    assert_eq!(
        ask_encoded,
        [60_000_000, 65_000_000, 75_000_000, 90_000_000, 95_000_000, 99_000_000]
    );
    assert_eq!(
        bid_encoded,
        [50_000_000, 45_000_000, 35_000_000, 30_000_000, 15_000_000, 10_000_000]
    );

    Ok(())
}
