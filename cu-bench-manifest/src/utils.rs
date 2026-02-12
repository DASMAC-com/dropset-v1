// Clippy: we intentionally hold a `RefCell` borrow across `.await`.
// This is safe only because this `Rc<RefCell<ProgramTestContext>>` is never used concurrently.
// Do not call helpers with the same `context` in parallel (e.g. `join!`, `spawn_local`).
#![allow(clippy::await_holding_refcell_ref)]

use std::{cell::RefCell, collections::HashMap, io::Error, rc::Rc};

use manifest::{
    deps::hypertree::{DataIndex, HyperTreeValueIteratorTrait as _},
    program::{
        batch_update::{CancelOrderParams, PlaceOrderParams},
        batch_update_instruction,
    },
    state::{OrderType, RestingOrder, NO_EXPIRATION_LAST_VALID_SLOT},
};
use solana_program::{hash::Hash, instruction::Instruction, pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::{signature::Keypair, transaction::Transaction};

use crate::{expand_market, send_tx_with_retry, TestFixture, Token, SOL_UNIT_SIZE, USDC_UNIT_SIZE};

pub const ONE_SOL: u64 = SOL_UNIT_SIZE;
pub const SLOT: u32 = NO_EXPIRATION_LAST_VALID_SLOT;
pub const N_WARMUP_ORDERS: u64 = 10;

/// Send a transaction and return the compute units consumed.
pub async fn send_tx_measure_cu(
    context: Rc<RefCell<ProgramTestContext>>,
    instructions: &[Instruction],
    payer: Option<&Pubkey>,
    signers: &[&Keypair],
) -> u64 {
    let mut context = context.borrow_mut();
    loop {
        let blockhash: Result<Hash, Error> = context.get_new_latest_blockhash().await;
        if blockhash.is_err() {
            continue;
        }
        let tx =
            Transaction::new_signed_with_payer(instructions, payer, signers, blockhash.unwrap());
        let result = context
            .banks_client
            .process_transaction_with_metadata(tx)
            .await
            .unwrap();
        if let Some(ref err) = result.result.err() {
            panic!("Transaction failed: {:?}", err);
        }
        let metadata = result.metadata.expect("metadata should be present");
        return metadata.compute_units_consumed;
    }
}

/// Reload the market and build a map of order_sequence_number -> data_index
/// for all resting orders on both sides of the book.
pub async fn collect_order_indices(test_fixture: &mut TestFixture) -> HashMap<u64, DataIndex> {
    test_fixture.market_fixture.reload().await;
    let market = &test_fixture.market_fixture.market;
    let mut map = HashMap::new();
    for (index, order) in market.get_asks().iter::<RestingOrder>() {
        map.insert(order.get_sequence_number(), index);
    }
    for (index, order) in market.get_bids().iter::<RestingOrder>() {
        map.insert(order.get_sequence_number(), index);
    }
    map
}

/// Reload the market and return the trader's seat data index.
pub async fn get_trader_index(test_fixture: &mut TestFixture, trader: &Pubkey) -> DataIndex {
    test_fixture.market_fixture.reload().await;
    test_fixture.market_fixture.market.get_trader_index(trader)
}

/// Warm up a market: claim seat, deposit both tokens, pre-expand, and place
/// initial orders on both sides of the book so that subsequent operations
/// hit the typical hot path (no expansion, non-empty book).
/// Returns (trader_index, order_indices) for use as hints.
pub async fn warm_up_market(
    test_fixture: &mut TestFixture,
) -> anyhow::Result<(DataIndex, HashMap<u64, DataIndex>)> {
    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    // Claim seat
    test_fixture.claim_seat().await?;

    // Deposit plenty of both tokens
    test_fixture
        .deposit(Token::SOL, 500 * SOL_UNIT_SIZE)
        .await?;
    test_fixture
        .deposit(Token::USDC, 500_000 * USDC_UNIT_SIZE)
        .await?;

    // Pre-expand market so there are plenty of free blocks (no expansion
    // during measured operations). 32 free blocks is well more than we need.
    expand_market(
        Rc::clone(&test_fixture.context),
        &test_fixture.market_fixture.key,
        32,
    )
    .await?;

    // Place orders on both sides to populate the book.
    let warmup_ix = batch_update_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        None,
        vec![],
        // Asks at prices 10-14
        vec![
            simple_bid(ONE_SOL, 10, 0),
            simple_bid(ONE_SOL, 11, 0),
            simple_bid(ONE_SOL, 12, 0),
            simple_bid(ONE_SOL, 13, 0),
            simple_bid(ONE_SOL, 14, 0),
        ],
        None,
        None,
        None,
        None,
    );
    send_tx_with_retry(
        Rc::clone(&test_fixture.context),
        &[warmup_ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await?;

    let warmup_ix = batch_update_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        None,
        vec![],
        // Bids at prices 1-5
        vec![
            simple_bid(ONE_SOL, 1, 0),
            simple_bid(ONE_SOL, 2, 0),
            simple_bid(ONE_SOL, 3, 0),
            simple_bid(ONE_SOL, 4, 0),
            simple_bid(ONE_SOL, 5, 0),
        ],
        None,
        None,
        None,
        None,
    );
    send_tx_with_retry(
        Rc::clone(&test_fixture.context),
        &[warmup_ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await?;

    // Collect hints for the measured operations.
    let trader_index = get_trader_index(test_fixture, &payer).await;
    let order_indices = collect_order_indices(test_fixture).await;

    assert_eq!(order_indices.len(), N_WARMUP_ORDERS as usize);

    Ok((trader_index, order_indices))
}

/// Create a fresh fixture, warm it up (seat, deposits, pre-expand, seed book),
/// and return (fixture, trader_index). Warmup places seq nums 0..=9.
pub async fn new_warmed_fixture() -> anyhow::Result<(TestFixture, DataIndex)> {
    let mut test_fixture: TestFixture = TestFixture::new().await;
    let (trader_index, _warmup_indices) = warm_up_market(&mut test_fixture).await?;
    Ok((test_fixture, trader_index))
}

/// Measure CU for a single instruction, with standard printing/signing.
pub async fn measure_ix(test_fixture: &TestFixture, label: &str, ix: Instruction) -> u64 {
    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;

    println!("{:<32} {:>6} CU", label, cu);
    cu
}

/// Build a BatchUpdate instruction.
pub fn batch_update_ix(
    test_fixture: &TestFixture,
    trader: &Pubkey,
    trader_index: Option<DataIndex>,
    cancels: Vec<CancelOrderParams>,
    places: Vec<PlaceOrderParams>,
) -> Instruction {
    batch_update_instruction(
        &test_fixture.market_fixture.key,
        trader,
        trader_index,
        cancels,
        places,
        None,
        None,
        None,
        None,
    )
}

/// Build a simple limit order with a last valid slot of 0 (no expiration).
fn simple_limit(
    base_atoms: u64,
    price_mantissa: u32,
    price_exponent: i8,
    is_bid: bool,
) -> PlaceOrderParams {
    PlaceOrderParams::new(
        base_atoms,
        price_mantissa,
        price_exponent,
        is_bid,
        OrderType::Limit,
        NO_EXPIRATION_LAST_VALID_SLOT,
    )
}

/// Build a simple limit bid order with a last valid slot of 0 (no expiration).
pub fn simple_bid(base_atoms: u64, price_mantissa: u32, price_exponent: i8) -> PlaceOrderParams {
    simple_limit(base_atoms, price_mantissa, price_exponent, true)
}

/// Build a simple limit ask order with a last valid slot of 0 (no expiration).
pub fn simple_ask(base_atoms: u64, price_mantissa: u32, price_exponent: i8) -> PlaceOrderParams {
    simple_limit(base_atoms, price_mantissa, price_exponent, false)
}
