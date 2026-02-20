use client::mollusk_helpers::{
    checks::IntoCheckFailure,
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use dropset_interface::{
    error::DropsetError,
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::NIL,
};
use mollusk_svm::result::Check;
use price::{
    to_order_info,
    OrderInfoArgs,
};
use solana_address::Address;

/// Verifies that post-only crossing checks fire in both directions, including at equal price:
/// - A bid whose price is at or above the best ask is rejected.
/// - An ask whose price is at or below the best bid is rejected.
#[test]
fn post_only_crossing_check() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    let ask = to_order_info(OrderInfoArgs::order_at_price(50_000_000)).unwrap();
    let bid = to_order_info(OrderInfoArgs::order_at_price(40_000_000)).unwrap();

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.quote.create_ata_idempotent(&user, &user),
            market_ctx.base.mint_to_owner(&user, ask.base_atoms)?,
            market_ctx.quote.mint_to_owner(&user, bid.quote_atoms)?,
        ])
        .program_result
        .is_ok());

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.deposit_base(user, ask.base_atoms, NIL),
            market_ctx.deposit_quote(user, bid.quote_atoms, 0),
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat after deposit");

    let post = |price, is_bid| {
        market_ctx.post_order(
            user,
            PostOrderInstructionData::new(OrderInfoArgs::order_at_price(price), is_bid, seat.index),
        )
    };
    let fail = || [DropsetError::PostOnlyWouldImmediatelyFill.into_check_failure()];

    let chain = [
        (post(50_000_000, false), [Check::success()]), // posting ask succeeds
        (post(50_000_001, true), fail()),              // bid above ask crosses
        (post(50_000_000, true), fail()),              // bid equal to ask crosses
        (post(40_000_000, true), [Check::success()]),  // posting bid succeeds
        (post(39_999_999, false), fail()),             // ask below bid crosses
        (post(40_000_000, false), fail()),             // ask equal to bid crosses
    ];
    let chain_refs: Vec<_> = chain.iter().map(|(i, c)| (i, c.as_slice())).collect();
    mollusk.process_and_validate_instruction_chain(&chain_refs);

    Ok(())
}

/// Verifies that a crossing failure clears after canceling the blocking order, but that the next
/// level of the book still blocks a more aggressive order.
#[test]
fn crossing_check_clears_with_cancel() -> anyhow::Result<()> {
    let user_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user = user_mock.0;
    let (mollusk, market_ctx) = new_dropset_mollusk_context_with_default_market(&[user_mock]);

    let ask_50 = to_order_info(OrderInfoArgs::order_at_price(50_000_000)).unwrap();
    let ask_60 = to_order_info(OrderInfoArgs::order_at_price(60_000_000)).unwrap();
    let bid_55 = to_order_info(OrderInfoArgs::order_at_price(55_000_000)).unwrap();

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user, &user),
            market_ctx.quote.create_ata_idempotent(&user, &user),
            market_ctx
                .base
                .mint_to_owner(&user, ask_50.base_atoms + ask_60.base_atoms)?,
            market_ctx.quote.mint_to_owner(&user, bid_55.quote_atoms)?,
        ])
        .program_result
        .is_ok());

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.deposit_base(user, ask_50.base_atoms + ask_60.base_atoms, NIL),
            market_ctx.deposit_quote(user, bid_55.quote_atoms, 0),
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat = market_ctx
        .find_seat(&market.seats, &user)
        .expect("User should have a seat");

    let post = |price, is_bid| {
        let data =
            PostOrderInstructionData::new(OrderInfoArgs::order_at_price(price), is_bid, seat.index);
        market_ctx.post_order(user, data)
    };
    let post_bid = |price| post(price, true);
    let post_ask = |price| post(price, false);
    let cancel = |price| {
        let data = CancelOrderInstructionData::new(price, false, seat.index);
        market_ctx.cancel_order(user, data)
    };
    let cross_failure = || DropsetError::PostOnlyWouldImmediatelyFill.into_check_failure();

    let chain = [
        (post_ask(50_000_000), [Check::success()]), // Ask at 50M
        (post_ask(60_000_000), [Check::success()]), // Ask at 60M
        (post_bid(55_000_000), [cross_failure()]),  // Bid at 55M fails because 50M ask exists
        (cancel(50_000_000), [Check::success()]),   // Cancel the 50M ask
        (post_bid(55_000_000), [Check::success()]), // Bid at 55M now clears
        (post_bid(65_000_000), [cross_failure()]),  // Bid at 65M still crosses 60M ask
    ];
    let chain_refs: Vec<_> = chain.iter().map(|(i, c)| (i, c.as_slice())).collect();
    mollusk.process_and_validate_instruction_chain(&chain_refs);

    Ok(())
}

/// Verifies that the crossing check is market-wide and not scoped to a single user's orders.
#[test]
fn crossing_check_across_users() -> anyhow::Result<()> {
    let user_a_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user_b_mock = create_mock_user_account(Address::new_unique(), 100_000_000);
    let user_a = user_a_mock.0;
    let user_b = user_b_mock.0;
    let (mollusk, market_ctx) =
        new_dropset_mollusk_context_with_default_market(&[user_a_mock, user_b_mock]);

    let ask = to_order_info(OrderInfoArgs::order_at_price(50_000_000)).unwrap();

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user_a, &user_a),
            market_ctx.base.mint_to_owner(&user_a, ask.base_atoms)?,
        ])
        .program_result
        .is_ok());

    assert!(mollusk
        .process_instruction_chain(&[market_ctx.deposit_base(user_a, ask.base_atoms, NIL)])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat_a = market_ctx
        .find_seat(&market.seats, &user_a)
        .expect("User A should have a seat");

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&user_b, &user_b),
            market_ctx.base.mint_to_owner(&user_b, 1)?,
        ])
        .program_result
        .is_ok());

    assert!(mollusk
        .process_instruction_chain(&[market_ctx.deposit_base(user_b, 1, NIL)])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    let seat_b = market_ctx
        .find_seat(&market.seats, &user_b)
        .expect("User B should have a seat");

    let chain = [
        (
            market_ctx.post_order(
                user_a,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(50_000_000),
                    false,
                    seat_a.index,
                ),
            ),
            [Check::success()],
        ),
        (
            market_ctx.post_order(
                user_b,
                PostOrderInstructionData::new(
                    OrderInfoArgs::order_at_price(50_000_001),
                    true,
                    seat_b.index,
                ),
            ),
            [DropsetError::PostOnlyWouldImmediatelyFill.into_check_failure()],
        ),
    ];
    let chain_refs: Vec<_> = chain.iter().map(|(i, c)| (i, c.as_slice())).collect();
    mollusk.process_and_validate_instruction_chain(&chain_refs);

    Ok(())
}
