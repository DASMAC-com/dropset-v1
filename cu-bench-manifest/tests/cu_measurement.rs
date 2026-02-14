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
    expand_market,
    new_fixture,
    send_tx_measure_cu,
    simple_ask,
    ONE_SOL,
    SOL_UNIT_SIZE,
    USDC_UNIT_SIZE,
};
use manifest::program::{
    batch_update::CancelOrderParams,
    deposit_instruction,
    swap_instruction,
    withdraw_instruction,
};
use solana_program_test::tokio;

const BATCH_AMOUNTS: &[u64] = &[1, 10, 50];
const SWAP_FILL_AMOUNTS: &[u64] = &[1, 10, 50];
const W: usize = 40;

/// Write a line centered within [`W`] characters.
fn wc(logs: &mut String, line: &str) {
    writeln!(logs, "{:^W$}", line).unwrap();
}

/// Write a `====== title ======` header line.
fn fmt_header(logs: &mut String, title: &str) {
    writeln!(logs, "\n{:=^W$}", format!(" {title} ")).unwrap();
}

/// Write a centered sub-table: column header, dashes, and data rows.
fn fmt_subtable(logs: &mut String, col_left: &str, rows: &[(u64, u64)]) {
    logs.push('\n');
    wc(logs, &format!("{:<14}{:>9}", col_left, "Average CU"));
    wc(logs, &"-".repeat(24));
    for &(n, avg) in rows {
        let label = format!("{n:>7} ");
        wc(logs, &format!("{label:<14}  {avg:>6}  "));
    }
}

const START_INDEX: u64 = 0;

// ── Single-instruction benchmarks ───────────────────────────────────────────

#[tokio::test]
async fn cu_deposit() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "Deposit");

    let (mut test_fixture, _trader_index) = new_fixture().await?;

    test_fixture
        .sol_mint_fixture
        .mint_to(&test_fixture.payer_sol_fixture.key, 10 * SOL_UNIT_SIZE)
        .await;

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();
    let ix = deposit_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        10 * SOL_UNIT_SIZE,
        &test_fixture.payer_sol_fixture.key,
        spl_token::id(),
        None,
    );

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    fmt_subtable(&mut logs, "Deposits", &[(1, cu)]);
    eprintln!("{logs}");
    Ok(())
}

#[tokio::test]
async fn cu_withdraw() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "Withdraw");

    let (test_fixture, _trader_index) = new_fixture().await?;

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();
    let ix = withdraw_instruction(
        &test_fixture.market_fixture.key,
        &payer,
        &test_fixture.sol_mint_fixture.key,
        ONE_SOL,
        &test_fixture.payer_sol_fixture.key,
        spl_token::id(),
        None,
    );

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    fmt_subtable(&mut logs, "Withdrawals", &[(1, cu)]);
    eprintln!("{logs}");
    Ok(())
}

// ── Batched benchmarks ──────────────────────────────────────────────────────

#[tokio::test]
async fn cu_batch_update_place() -> anyhow::Result<()> {
    let mut logs = String::new();
    for pre_expand in [false, true] {
        fmt_header(&mut logs, "BatchUpdate (Place)");
        if pre_expand {
            wc(&mut logs, "[pre-expanded]")
        };
        let mut rows = Vec::new();
        for &n in BATCH_AMOUNTS {
            rows.push((n, batch_place(n, pre_expand).await?));
        }
        fmt_subtable(&mut logs, "Orders", &rows);
    }
    eprintln!("{logs}");
    Ok(())
}

async fn batch_place(n: u64, pre_expand: bool) -> anyhow::Result<u64> {
    let (test_fixture, trader_index) = new_fixture().await?;

    if pre_expand {
        expand_market(
            Rc::clone(&test_fixture.context),
            &test_fixture.market_fixture.key,
            n as u32 * 2,
        )
        .await?;
    }

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    let places: Vec<_> = (0..n)
        .map(|i| simple_ask(ONE_SOL, 15 + i as u32, 0))
        .collect();

    let ix = batch_update_ix(&test_fixture, &payer, Some(trader_index), vec![], places);

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    Ok(cu / n)
}

#[tokio::test]
async fn cu_batch_update_cancel() -> anyhow::Result<()> {
    let mut logs = String::new();
    for pre_expand in [false, true] {
        fmt_header(&mut logs, "BatchUpdate (Cancel)");
        if pre_expand {
            wc(&mut logs, "[pre-expanded]")
        };
        let mut rows = Vec::new();
        for &n in BATCH_AMOUNTS {
            rows.push((n, batch_cancel(n, pre_expand).await?));
        }
        fmt_subtable(&mut logs, "Cancels", &rows);
    }
    eprintln!("{logs}");
    Ok(())
}

async fn batch_cancel(n: u64, pre_expand: bool) -> anyhow::Result<u64> {
    let (mut test_fixture, trader_index) = new_fixture().await?;

    if pre_expand {
        expand_market(
            Rc::clone(&test_fixture.context),
            &test_fixture.market_fixture.key,
            n as u32 * 2,
        )
        .await?;
    }

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    // Place n orders first.
    let places: Vec<_> = (0..n)
        .map(|i| simple_ask(ONE_SOL, 15 + i as u32, 0))
        .collect();

    let place_ix = batch_update_ix(&test_fixture, &payer, Some(trader_index), vec![], places);
    send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[place_ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;

    // Cancel all n orders.
    let order_indices = collect_order_indices(&mut test_fixture).await;
    let cancels: Vec<_> = (START_INDEX..START_INDEX + n)
        .map(|i| CancelOrderParams::new_with_hint(i, Some(order_indices[&i])))
        .collect();

    let ix = batch_update_ix(&test_fixture, &payer, Some(trader_index), cancels, vec![]);

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    Ok(cu / n)
}

#[tokio::test]
async fn cu_batch_update_cancel_and_place() -> anyhow::Result<()> {
    let mut logs = String::new();
    for pre_expand in [false, true] {
        fmt_header(&mut logs, "BatchUpdate (Cancel+Place)");
        if pre_expand {
            wc(&mut logs, "[pre-expanded]")
        };
        let mut rows = Vec::new();
        for &n in BATCH_AMOUNTS {
            rows.push((n, batch_place_and_cancel(n, pre_expand).await?));
        }
        fmt_subtable(&mut logs, "Orders", &rows);
    }
    eprintln!("{logs}");
    Ok(())
}

async fn batch_place_and_cancel(n: u64, pre_expand: bool) -> anyhow::Result<u64> {
    let (mut test_fixture, trader_index) = new_fixture().await?;

    if pre_expand {
        expand_market(
            Rc::clone(&test_fixture.context),
            &test_fixture.market_fixture.key,
            n as u32 * 2,
        )
        .await?;
    }

    let payer = test_fixture.payer();
    let payer_keypair = test_fixture.payer_keypair();

    // Place n orders first (setup).
    let initial_places: Vec<_> = (0..n)
        .map(|i| simple_ask(ONE_SOL, 15 + i as u32, 0))
        .collect();

    let place_ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        vec![],
        initial_places,
    );
    send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[place_ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;

    // Cancel n + place n in one instruction.
    let order_indices = collect_order_indices(&mut test_fixture).await;
    let cancels: Vec<_> = (START_INDEX..START_INDEX + n)
        .map(|i| CancelOrderParams::new_with_hint(i, Some(order_indices[&i])))
        .collect();
    let new_places: Vec<_> = (0..n)
        .map(|i| simple_ask(ONE_SOL, 100 + i as u32, 0))
        .collect();

    let ix = batch_update_ix(
        &test_fixture,
        &payer,
        Some(trader_index),
        cancels,
        new_places,
    );

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    // Divide by n (not 2n) since each "unit of work" is a cancel+place pair.
    Ok(cu / n)
}

// ── Swap benchmarks ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cu_swap() -> anyhow::Result<()> {
    let mut logs = String::new();
    for pre_expand in [false, true] {
        fmt_header(&mut logs, "Swap");
        if pre_expand {
            wc(&mut logs, "[pre-expanded]")
        };
        let mut rows = Vec::new();
        for &n in SWAP_FILL_AMOUNTS {
            rows.push((n, swap_fill(n, pre_expand).await?));
        }
        fmt_subtable(&mut logs, "Fills", &rows);
    }
    eprintln!("{logs}");
    Ok(())
}

async fn swap_fill(n: u64, pre_expand: bool) -> anyhow::Result<u64> {
    let (mut test_fixture, trader_index) = new_fixture().await?;

    if pre_expand {
        expand_market(
            Rc::clone(&test_fixture.context),
            &test_fixture.market_fixture.key,
            n as u32 * 2,
        )
        .await?;
    }

    let payer_keypair = test_fixture.payer_keypair();

    // Place n resting asks.
    if n > 1 {
        let extra_asks: Vec<_> = (0..n)
            .map(|i| simple_ask(ONE_SOL, 10 + i as u32, 0))
            .collect();
        test_fixture
            .batch_update_for_keypair(Some(trader_index), vec![], extra_asks, &payer_keypair)
            .await?;
    }

    // Fund the taker with USDC.
    test_fixture
        .usdc_mint_fixture
        .mint_to(
            &test_fixture.payer_usdc_fixture.key,
            1_000_000 * USDC_UNIT_SIZE,
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
        n * SOL_UNIT_SIZE,
        0,
        false, // quote (USDC) is input.
        true,
        spl_token::id(),
        spl_token::id(),
        false,
    );

    let cu = send_tx_measure_cu(
        Rc::clone(&test_fixture.context),
        &[ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await;
    Ok(cu / n)
}
