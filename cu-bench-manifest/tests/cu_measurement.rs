// Clippy: we intentionally hold a `RefCell` borrow across `.await`.
// This is safe only because this `Rc<RefCell<ProgramTestContext>>` is never used concurrently.
// Do not call helpers with the same `context` in parallel (e.g. `join!`, `spawn_local`).
#![allow(clippy::await_holding_refcell_ref)]

use std::{
    fmt::Write,
    rc::Rc,
};

use cu_bench_manifest::{
    batch_update_ix,
    collect_order_indices,
    measure_ix,
    new_warmed_fixture,
    send_tx_measure_cu,
    simple_ask,
    N_WARMUP_ORDERS,
    ONE_SOL,
    SOL_UNIT_SIZE,
    USDC_UNIT_SIZE,
};
use manifest::program::{
    batch_update::{
        CancelOrderParams,
        PlaceOrderParams,
    },
    deposit_instruction,
    swap_instruction,
    withdraw_instruction,
};
use solana_program_test::tokio;

#[tokio::test]
async fn cu_deposit() -> anyhow::Result<()> {
    let (mut test_fixture, _trader_index) = new_warmed_fixture().await?;

    test_fixture
        .sol_mint_fixture
        .mint_to(&test_fixture.payer_sol_fixture.key, 10 * SOL_UNIT_SIZE)
        .await;

    let payer = test_fixture.payer();
    let deposit_ix = deposit_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        10 * SOL_UNIT_SIZE,
        &test_fixture.payer_sol_fixture.key,
        spl_token::id(),
        None,
    );

    println!("\n========== CU: Deposit ==========");
    measure_ix(&test_fixture, "Deposit", deposit_ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_batch_update_place_1() -> anyhow::Result<()> {
    let (test_fixture, trader_index) = new_warmed_fixture().await?;

    let payer = test_fixture.payer();
    let ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        vec![],
        vec![simple_ask(ONE_SOL, 15, 0)],
    );

    println!("\n========== CU: BatchUpdate (place 1) ==========");
    measure_ix(&test_fixture, "BatchUpdate (place 1)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_batch_update_cancel_1() -> anyhow::Result<()> {
    let (mut test_fixture, trader_index) = new_warmed_fixture().await?;

    let payer_keypair = test_fixture.payer_keypair();

    // Place one order so we have something to cancel.
    test_fixture
        .batch_update_for_keypair(
            Some(trader_index),
            vec![],
            vec![simple_ask(ONE_SOL, 15, 0)],
            &payer_keypair,
        )
        .await?;

    let order_indices = collect_order_indices(&mut test_fixture).await;
    let payer = test_fixture.payer();
    let ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        vec![CancelOrderParams::new_with_hint(
            N_WARMUP_ORDERS,
            Some(order_indices[&N_WARMUP_ORDERS]),
        )],
        vec![],
    );

    println!("\n========== CU: BatchUpdate (cancel 1) ==========");
    measure_ix(&test_fixture, "BatchUpdate (cancel 1)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_batch_update_cancel_1_place_1() -> anyhow::Result<()> {
    let (mut test_fixture, trader_index) = new_warmed_fixture().await?;

    let payer_keypair = test_fixture.payer_keypair();

    // Place one order to cancel.
    test_fixture
        .batch_update_for_keypair(
            Some(trader_index),
            vec![],
            vec![simple_ask(ONE_SOL, 15, 0)],
            &payer_keypair,
        )
        .await?;

    let order_indices = collect_order_indices(&mut test_fixture).await;
    let payer = test_fixture.payer();
    let ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        vec![CancelOrderParams::new_with_hint(
            N_WARMUP_ORDERS,
            Some(order_indices[&N_WARMUP_ORDERS]),
        )],
        vec![simple_ask(ONE_SOL, 16, 0)],
    );

    println!("\n========== CU: BatchUpdate (cancel 1 + place 1) ==========");
    measure_ix(&test_fixture, "BatchUpdate (cancel 1 + place 1)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_batch_update_cancel_4_place_4() -> anyhow::Result<()> {
    let (mut test_fixture, trader_index) = new_warmed_fixture().await?;

    let payer_keypair = test_fixture.payer_keypair();

    // Place 4 orders so we have known seq nums to cancel.
    test_fixture
        .batch_update_for_keypair(
            Some(trader_index),
            vec![],
            vec![
                simple_ask(ONE_SOL, 20, 0),
                simple_ask(ONE_SOL, 21, 0),
                simple_ask(ONE_SOL, 22, 0),
                simple_ask(ONE_SOL, 23, 0),
            ],
            &payer_keypair,
        )
        .await?;

    let order_indices = collect_order_indices(&mut test_fixture).await;
    let payer = test_fixture.payer();
    let ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        // The canceled orders' sequence numbers and indices are the same (for each order).
        // The first canceled order begins at the number of warmup orders already placed.
        (N_WARMUP_ORDERS..=N_WARMUP_ORDERS + 3)
            .map(|i| CancelOrderParams::new_with_hint(i, Some(order_indices[&i])))
            .collect(),
        vec![
            simple_ask(ONE_SOL, 30, 0),
            simple_ask(ONE_SOL, 31, 0),
            simple_ask(ONE_SOL, 32, 0),
            simple_ask(ONE_SOL, 33, 0),
        ],
    );

    println!("\n========== CU: BatchUpdate (cancel 4 + place 4) ==========");
    measure_ix(&test_fixture, "BatchUpdate (cancel 4 + place 4)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_swap_fill_1() -> anyhow::Result<()> {
    let (mut test_fixture, _trader_index) = new_warmed_fixture().await?;

    println!("\n========== CU: Swap (fill 1 order) ==========");

    // Ensure payer has plenty of USDC.
    test_fixture
        .usdc_mint_fixture
        .mint_to(
            &test_fixture.payer_usdc_fixture.key,
            100_000 * USDC_UNIT_SIZE,
        )
        .await;

    let payer = test_fixture.payer();
    let ix = swap_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        &test_fixture.usdc_mint_fixture.key,
        &test_fixture.payer_sol_fixture.key,
        &test_fixture.payer_usdc_fixture.key,
        ONE_SOL,
        0,
        false, // quote (USDC) is input
        true,  // is_exact_in
        spl_token::id(),
        spl_token::id(),
        false,
    );

    measure_ix(&test_fixture, "Swap (fill 1 order)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_swap_fill_3() -> anyhow::Result<()> {
    let (mut test_fixture, trader_index) = new_warmed_fixture().await?;

    println!("\n========== CU: Swap (fill 3 orders) ==========");

    let payer_keypair = test_fixture.payer_keypair();

    // Add 3 more asks at the same price so the swap walks through several.
    test_fixture
        .batch_update_for_keypair(
            Some(trader_index),
            vec![],
            vec![
                simple_ask(ONE_SOL, 10, 0),
                simple_ask(ONE_SOL, 10, 0),
                simple_ask(ONE_SOL, 10, 0),
            ],
            &payer_keypair,
        )
        .await?;

    // Ensure payer has plenty of USDC.
    test_fixture
        .usdc_mint_fixture
        .mint_to(
            &test_fixture.payer_usdc_fixture.key,
            100_000 * USDC_UNIT_SIZE,
        )
        .await;

    let payer = test_fixture.payer();
    let ix = swap_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        &test_fixture.usdc_mint_fixture.key,
        &test_fixture.payer_sol_fixture.key,
        &test_fixture.payer_usdc_fixture.key,
        3 * SOL_UNIT_SIZE,
        0,
        false,
        true,
        spl_token::id(),
        spl_token::id(),
        false,
    );

    measure_ix(&test_fixture, "Swap (fill 3 orders)", ix).await;
    Ok(())
}

#[tokio::test]
async fn cu_withdraw() -> anyhow::Result<()> {
    let (test_fixture, _trader_index) = new_warmed_fixture().await?;

    println!("\n========== CU: Withdraw ==========");

    let payer = test_fixture.payer();
    let ix = withdraw_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        ONE_SOL,
        &test_fixture.payer_sol_fixture.key,
        spl_token::id(),
        None,
    );

    measure_ix(&test_fixture, "Withdraw", ix).await;
    Ok(())
}

// -------------------------
// Maker spam test (kept, lightly refactored)
// -------------------------

#[tokio::test]
async fn measure_many_maker_batch_replace() -> anyhow::Result<()> {
    const N_ORDERS_PER_BATCH: u64 = 4;
    const N_BATCHES: u64 = 5;

    let cu_logs = &mut String::new();

    let (mut test_fixture, trader_index) = new_warmed_fixture().await?;

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    writeln!(cu_logs, "\n========== Maker spam: cancel {N_ORDERS_PER_BATCH} + place {N_ORDERS_PER_BATCH}, {N_BATCHES} times ==========")?;
    writeln!(cu_logs, "(Market pre-warmed, book has 5 asks + 5 bids)\n")?;

    // Place N_ORDERS_PER_BATCH initial asks to kick things off.
    test_fixture
        .batch_update_for_keypair(
            Some(trader_index),
            vec![],
            vec![
                simple_ask(ONE_SOL, 20, 0),
                simple_ask(ONE_SOL, 21, 0),
                simple_ask(ONE_SOL, 22, 0),
                simple_ask(ONE_SOL, 23, 0),
            ],
            &payer_keypair,
        )
        .await?;

    let mut prev_seq_nums: Vec<u64> =
        (N_WARMUP_ORDERS..N_WARMUP_ORDERS + N_ORDERS_PER_BATCH).collect();
    let mut total_cu: u64 = 0;

    for round in 0..N_BATCHES {
        let base_price = 30 + round * 10;

        // Look up data indices for the orders we're about to cancel.
        let order_indices = collect_order_indices(&mut test_fixture).await;

        let cancels: Vec<CancelOrderParams> = prev_seq_nums
            .iter()
            .map(|&seq| CancelOrderParams::new_with_hint(seq, Some(order_indices[&seq])))
            .collect();

        let places: Vec<PlaceOrderParams> = (0..4)
            .map(|i| simple_ask(ONE_SOL, (base_price + i) as u32, 0))
            .collect();

        let ix = batch_update_ix(&test_fixture, &payer, Some(trader_index), cancels, places);

        let cu = send_tx_measure_cu(
            Rc::clone(&test_fixture.context),
            &[ix],
            Some(&payer),
            &[&payer_keypair],
        )
        .await;

        // Each round places N_ORDERS_PER_BATCH new orders.
        // The N_ORDERS_PER_BATCH initial asks add an additional N_ORDERS_PER_BATCH offset.
        let first_seq = N_WARMUP_ORDERS + (round + 1) * N_ORDERS_PER_BATCH;
        prev_seq_nums = (first_seq..first_seq + N_ORDERS_PER_BATCH).collect();

        total_cu += cu;
        writeln!(
            cu_logs,
            "  Round {} (cancel {N_ORDERS_PER_BATCH} + place {N_ORDERS_PER_BATCH})   {:>6} CU",
            round + 1,
            cu
        )?;
    }

    writeln!(
        cu_logs,
        "  TOTAL  ({N_BATCHES} rounds)              {:>6} CU",
        total_cu
    )?;
    writeln!(
        cu_logs,
        "  Average per round              {:>6} CU",
        total_cu / N_BATCHES
    )?;

    writeln!(cu_logs, "\n{}", "=".repeat(60))?;

    print!("{cu_logs}");

    Ok(())
}
