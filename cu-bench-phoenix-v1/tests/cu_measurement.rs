use std::fmt::Write;

use cu_bench_phoenix_v1::{
    clone_keypair,
    create_ata_pub,
    ioc_buy,
    mint_to_pub,
    new_initialized_fixture,
    send_txn,
    send_txn_measure_cu,
    simple_post_only_ask,
    NUM_BASE_LOTS_PER_BASE_UNIT,
    QUOTE_UNIT,
};
use phoenix::program::{
    deposit::DepositParams,
    instruction_builders::*,
    new_order::{
        CondensedOrder,
        FailedMultipleLimitOrderBehavior,
        MultipleOrderPacket,
    },
};
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

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

// ── Single-instruction benchmarks ───────────────────────────────────────────

#[tokio::test]
async fn cu_deposit() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "Deposit");

    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    let ix = create_deposit_funds_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &DepositParams {
            quote_lots_to_deposit: 0,
            base_lots_to_deposit: 10 * NUM_BASE_LOTS_PER_BASE_UNIT,
        },
    );

    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&maker]).await;
    fmt_subtable(&mut logs, "Deposits", &[(1, cu)]);
    eprintln!("{logs}");
    Ok(())
}

#[tokio::test]
async fn cu_withdraw() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "Withdraw");

    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    let ix =
        create_withdraw_funds_instruction(&f.market, &maker.pubkey(), &f.base_mint, &f.quote_mint);

    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&maker]).await;
    fmt_subtable(&mut logs, "Withdrawals", &[(1, cu)]);
    eprintln!("{logs}");
    Ok(())
}

// ── Batched benchmarks ──────────────────────────────────────────────────────

#[tokio::test]
async fn cu_place_limit_order() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "PlaceLimitOrder");

    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    let order = simple_post_only_ask(1600, 10);
    let ix = create_new_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &order,
    );

    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&maker]).await;
    fmt_subtable(&mut logs, "Orders", &[(1, cu)]);
    eprintln!("{logs}");
    Ok(())
}

#[tokio::test]
async fn cu_place_multiple_post_only() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "PlaceMultiplePostOnly");
    let mut rows = Vec::new();
    for &n in BATCH_AMOUNTS {
        rows.push((n, place_multiple_post_only(n).await?));
    }
    fmt_subtable(&mut logs, "Orders", &rows);
    eprintln!("{logs}");
    Ok(())
}

async fn place_multiple_post_only(n: u64) -> anyhow::Result<u64> {
    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    let asks: Vec<CondensedOrder> = (0..n)
        .map(|i| CondensedOrder {
            price_in_ticks: 2000 + i * 100,
            size_in_base_lots: 10,
            last_valid_slot: None,
            last_valid_unix_timestamp_in_seconds: None,
        })
        .collect();

    let ix = create_new_multiple_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &MultipleOrderPacket {
            bids: vec![],
            asks,
            client_order_id: None,
            failed_multiple_limit_order_behavior:
                FailedMultipleLimitOrderBehavior::FailOnInsufficientFundsAndFailOnCross,
        },
    );

    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&maker]).await;
    Ok(cu / n)
}

#[tokio::test]
async fn cu_cancel_all() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "CancelAllOrders");
    let mut rows = Vec::new();
    for &n in BATCH_AMOUNTS {
        rows.push((n, cancel_all(n).await?));
    }
    fmt_subtable(&mut logs, "Cancels", &rows);
    eprintln!("{logs}");
    Ok(())
}

async fn cancel_all(n: u64) -> anyhow::Result<u64> {
    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    let asks: Vec<CondensedOrder> = (0..n)
        .map(|i| CondensedOrder {
            price_in_ticks: 1100 + i * 100,
            size_in_base_lots: 10,
            last_valid_slot: None,
            last_valid_unix_timestamp_in_seconds: None,
        })
        .collect();

    let place_ix = create_new_multiple_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &MultipleOrderPacket {
            bids: vec![],
            asks,
            client_order_id: None,
            failed_multiple_limit_order_behavior:
                FailedMultipleLimitOrderBehavior::FailOnInsufficientFundsAndFailOnCross,
        },
    );
    send_txn(&mut f.context, &[place_ix], &[&maker]).await;

    let ix = create_cancel_all_order_with_free_funds_instruction(&f.market, &maker.pubkey());
    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&maker]).await;
    Ok(cu / n)
}

// ── Swap benchmarks ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cu_swap() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "1 Swap, N fills");
    let mut rows = Vec::new();
    for &n in SWAP_FILL_AMOUNTS {
        rows.push((n, swap_fill(n).await?));
    }
    fmt_subtable(&mut logs, "Fills", &rows);
    eprintln!("{logs}");
    Ok(())
}

async fn swap_fill(n: u64) -> anyhow::Result<u64> {
    let mut f = new_initialized_fixture().await?;

    let payer = f.payer_keypair();
    let maker = f.maker_keypair();

    create_ata_pub(&mut f.context, &payer.pubkey(), &f.base_mint).await;
    let payer_quote_ata =
        spl_associated_token_account::get_associated_token_address(&payer.pubkey(), &f.quote_mint);
    let mint_auth = clone_keypair(&f.mint_authority);
    mint_to_pub(
        &mut f.context,
        &mint_auth,
        &f.quote_mint,
        &payer_quote_ata,
        1_000_000 * QUOTE_UNIT,
    )
    .await;

    // Place n resting asks at ascending prices.
    let asks: Vec<CondensedOrder> = (0..n)
        .map(|i| CondensedOrder {
            price_in_ticks: 1100 + i * 10,
            size_in_base_lots: 10,
            last_valid_slot: None,
            last_valid_unix_timestamp_in_seconds: None,
        })
        .collect();

    let place_ix = create_new_multiple_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &MultipleOrderPacket {
            bids: vec![],
            asks,
            client_order_id: None,
            failed_multiple_limit_order_behavior:
                FailedMultipleLimitOrderBehavior::FailOnInsufficientFundsAndFailOnCross,
        },
    );
    send_txn(&mut f.context, &[place_ix], &[&maker]).await;

    // IOC buy that crosses all n resting asks.
    let max_price = 1100 + n * 10 + 100;
    let total_base_lots = n * 10;
    let order = ioc_buy(max_price, total_base_lots);
    let ix = create_new_order_instruction(
        &f.market,
        &payer.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &order,
    );

    let cu = send_txn_measure_cu(&mut f.context, &[ix], &[&payer]).await;
    Ok(cu / n)
}

// ── MultipleOrderPacket (Batch): place then cancel ──────────────────────────

#[tokio::test]
async fn cu_multiple_order_packet_batch() -> anyhow::Result<()> {
    let mut logs = String::new();
    fmt_header(&mut logs, "MultipleOrderPacket (Batch)");

    let mut place_rows = Vec::new();
    let mut cancel_rows = Vec::new();
    for &n in BATCH_AMOUNTS {
        let (place_avg, cancel_avg) = batch_place_then_cancel(n).await?;
        place_rows.push((n, place_avg));
        cancel_rows.push((n, cancel_avg));
    }

    fmt_subtable(&mut logs, "Places", &place_rows);
    fmt_subtable(&mut logs, "Cancels", &cancel_rows);
    eprintln!("{logs}");
    Ok(())
}

async fn batch_place_then_cancel(n: u64) -> anyhow::Result<(u64, u64)> {
    let mut f = new_initialized_fixture().await?;
    let maker = f.maker_keypair();

    // Place n orders via MultipleOrderPacket.
    let asks: Vec<CondensedOrder> = (0..n)
        .map(|i| CondensedOrder {
            price_in_ticks: 2000 + i * 10,
            size_in_base_lots: 10,
            last_valid_slot: None,
            last_valid_unix_timestamp_in_seconds: None,
        })
        .collect();

    let place_ix = create_new_multiple_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &MultipleOrderPacket {
            bids: vec![],
            asks,
            client_order_id: None,
            failed_multiple_limit_order_behavior:
                FailedMultipleLimitOrderBehavior::FailOnInsufficientFundsAndFailOnCross,
        },
    );
    let place_cu = send_txn_measure_cu(&mut f.context, &[place_ix], &[&maker]).await;

    // Cancel all.
    let cancel_ix = create_cancel_all_order_with_free_funds_instruction(&f.market, &maker.pubkey());
    let cancel_cu = send_txn_measure_cu(&mut f.context, &[cancel_ix], &[&maker]).await;

    Ok((place_cu / n, cancel_cu / n))
}
