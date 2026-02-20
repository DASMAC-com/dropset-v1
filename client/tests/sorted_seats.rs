use client::mollusk_helpers::{
    helper_trait::DropsetTestHelper,
    new_dropset_mollusk_context_with_default_market,
    utils::create_mock_user_account,
};
use dropset_interface::state::sector::NIL;
use solana_sdk::pubkey;

const ADDR_A: solana_address::Address = pubkey!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
const ADDR_B: solana_address::Address = pubkey!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");
const ADDR_C: solana_address::Address = pubkey!("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC");

#[test]
fn two_seats() -> anyhow::Result<()> {
    let user_a_mock = create_mock_user_account(ADDR_A, 100_000_000);
    let user_b_mock = create_mock_user_account(ADDR_B, 100_000_000);
    let user_c_mock = create_mock_user_account(ADDR_C, 100_000_000);
    let (mollusk, market_ctx) =
        new_dropset_mollusk_context_with_default_market(&[user_a_mock, user_b_mock, user_c_mock]);

    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.base.create_ata_idempotent(&ADDR_A, &ADDR_A),
            market_ctx.base.create_ata_idempotent(&ADDR_B, &ADDR_B),
            market_ctx.base.create_ata_idempotent(&ADDR_C, &ADDR_C),
            market_ctx.base.mint_to_owner(&ADDR_A, 1_000)?,
            market_ctx.base.mint_to_owner(&ADDR_B, 1_000)?,
            market_ctx.base.mint_to_owner(&ADDR_C, 1_000)?,
        ])
        .program_result
        .is_ok());

    // Deposit B first, then C, then A.
    // This verifies that the DLL properly sorts new seats regardless of the insertion order.
    assert!(mollusk
        .process_instruction_chain(&[
            market_ctx.deposit_base(ADDR_B, 100, NIL),
            market_ctx.deposit_base(ADDR_C, 200, NIL),
            market_ctx.deposit_base(ADDR_A, 300, NIL)
        ])
        .program_result
        .is_ok());

    let market = mollusk.view_market(&market_ctx.market);
    assert_eq!(market.seats.len(), 3);
    assert_eq!(market.seats[0].user, ADDR_A);
    assert_eq!(market.seats[0].base_available, 300);
    assert_eq!(market.seats[1].user, ADDR_B);
    assert_eq!(market.seats[1].base_available, 100);
    assert_eq!(market.seats[2].user, ADDR_C);
    assert_eq!(market.seats[2].base_available, 200);

    Ok(())
}
