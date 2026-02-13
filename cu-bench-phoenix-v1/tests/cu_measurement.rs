use std::fmt::Write;

use cu_bench_phoenix_v1::{
    clone_keypair,
    create_ata_pub,
    ioc_buy,
    measure_ixn,
    mint_to_pub,
    new_warmed_fixture,
    send_tx_measure_cu,
    simple_post_only_ask,
    NUM_BASE_LOTS_PER_BASE_UNIT,
    NUM_INITIAL_ORDERS_PER_SIDE,
    QUOTE_UNIT,
};
use phoenix::program::{
    deposit::DepositParams,
    instruction_builders::*,
    new_order::{
        CondensedOrder,
        MultipleOrderPacket,
    },
};
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

#[tokio::test]
async fn cu_deposit() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;
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

    writeln!(f.logs, "\n========== Instruction: Deposit ==========").unwrap();
    measure_ixn(&mut f, ix, 1, maker).await;
    Ok(())
}

#[tokio::test]
async fn cu_place_limit_order_1() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;
    let maker = f.maker_keypair();

    let order = simple_post_only_ask(1600, 10);
    let ix = create_new_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &order,
    );

    writeln!(
        f.logs,
        "\n========== Instruction: PlaceLimitOrder (1) =========="
    )
    .unwrap();
    measure_ixn(&mut f, ix, 1, maker).await;
    Ok(())
}

#[tokio::test]
async fn cu_place_multiple_4() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;
    let maker = f.maker_keypair();

    let ix = create_new_multiple_order_instruction(
        &f.market,
        &maker.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &MultipleOrderPacket {
            bids: vec![],
            asks: vec![
                CondensedOrder {
                    price_in_ticks: 2000,
                    size_in_base_lots: 10,
                },
                CondensedOrder {
                    price_in_ticks: 2100,
                    size_in_base_lots: 10,
                },
                CondensedOrder {
                    price_in_ticks: 2200,
                    size_in_base_lots: 10,
                },
                CondensedOrder {
                    price_in_ticks: 2300,
                    size_in_base_lots: 10,
                },
            ],
            client_order_id: None,
            reject_post_only: true,
        },
    );

    writeln!(
        f.logs,
        "\n========== Instruction: PlaceMultiplePostOnly (4) =========="
    )
    .unwrap();
    measure_ixn(&mut f, ix, 4, maker).await;
    Ok(())
}

#[tokio::test]
async fn cu_cancel_all() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;
    let maker = f.maker_keypair();

    let ix = create_cancel_all_order_with_free_funds_instruction(&f.market, &maker.pubkey());

    writeln!(
        f.logs,
        "\n========== Instruction: CancelAllOrders ({}) ==========",
        NUM_INITIAL_ORDERS_PER_SIDE * 2
    )
    .unwrap();
    measure_ixn(&mut f, ix, NUM_INITIAL_ORDERS_PER_SIDE * 2, maker).await;
    Ok(())
}

#[tokio::test]
async fn cu_swap_fill_1() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;

    // Use the payer as the taker.
    let payer = f.payer_keypair();

    // Create payer's base ATA and fund payer's quote ATA.
    create_ata_pub(&mut f.context, &payer.pubkey(), &f.base_mint).await;
    let payer_quote_ata =
        spl_associated_token_account::get_associated_token_address(&payer.pubkey(), &f.quote_mint);
    let mint_auth = clone_keypair(&f.mint_authority);
    mint_to_pub(
        &mut f.context,
        &mint_auth,
        &f.quote_mint,
        &payer_quote_ata,
        100_000 * QUOTE_UNIT,
    )
    .await;

    // IOC buy: fill 1 resting ask. Warmup asks at ticks 1100-1500, 10 base lots each.
    // Buy 10 base lots at up to tick 1200 → fills the ask at 1100.
    let order = ioc_buy(1200, 10);
    let ix = create_new_order_instruction(
        &f.market,
        &payer.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &order,
    );

    writeln!(
        f.logs,
        "\n========== Instruction: Swap (fill 1 order) =========="
    )
    .unwrap();
    measure_ixn(&mut f, ix, 1, payer).await;
    Ok(())
}

#[tokio::test]
async fn cu_swap_fill_3() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;

    let payer = f.payer_keypair();

    // Create payer's base ATA and fund quote ATA.
    create_ata_pub(&mut f.context, &payer.pubkey(), &f.base_mint).await;
    let payer_quote_ata =
        spl_associated_token_account::get_associated_token_address(&payer.pubkey(), &f.quote_mint);
    let mint_auth = clone_keypair(&f.mint_authority);
    mint_to_pub(
        &mut f.context,
        &mint_auth,
        &f.quote_mint,
        &payer_quote_ata,
        100_000 * QUOTE_UNIT,
    )
    .await;

    // IOC buy: fill 3 resting asks (at ticks 1100, 1200, 1300).
    // Buy 30 base lots at up to tick 1400.
    let order = ioc_buy(1400, 30);
    let ix = create_new_order_instruction(
        &f.market,
        &payer.pubkey(),
        &f.base_mint,
        &f.quote_mint,
        &order,
    );

    writeln!(
        f.logs,
        "\n========== Instruction: Swap (fill 3 orders) =========="
    )
    .unwrap();
    measure_ixn(&mut f, ix, 3, payer).await;
    Ok(())
}

#[tokio::test]
async fn cu_withdraw() -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;
    let maker = f.maker_keypair();

    let ix =
        create_withdraw_funds_instruction(&f.market, &maker.pubkey(), &f.base_mint, &f.quote_mint);

    writeln!(f.logs, "\n========== Instruction: Withdraw ==========").unwrap();
    measure_ixn(&mut f, ix, 1, maker).await;
    Ok(())
}

// ── Maker spam test ─────────────────────────────────────────────────────────

#[tokio::test]
async fn measure_several_maker_cancel_replace() -> anyhow::Result<()> {
    maker_cancel_replace(100, 5).await
}

#[tokio::test]
async fn measure_many_maker_cancel_replace() -> anyhow::Result<()> {
    maker_cancel_replace(10, 5).await
}

async fn maker_cancel_replace(n_orders: usize, n_rounds: usize) -> anyhow::Result<()> {
    let mut f = new_warmed_fixture().await?;

    writeln!(
        &mut f.logs,
        "\n========== Maker spam: cancel all + place {n_orders}, {n_rounds} times =========="
    )?;
    writeln!(
        &mut f.logs,
        "(Market pre-warmed, book has 5 asks + 5 bids)\n"
    )?;

    let mut num_existing_orders = NUM_INITIAL_ORDERS_PER_SIDE * 2;
    let mut total_cancel_cu = 0;
    let mut total_num_cancels = NUM_INITIAL_ORDERS_PER_SIDE;
    let mut total_place_cu = 0;
    let mut total_num_places = NUM_INITIAL_ORDERS_PER_SIDE;

    for round in 0..n_rounds {
        let maker = f.maker_keypair();
        let base_tick = 2000 + (round as u64) * 100;

        // Cancel all existing orders (free funds variant).
        let cancel_ix =
            create_cancel_all_order_with_free_funds_instruction(&f.market, &maker.pubkey());
        let cancel_cu = send_tx_measure_cu(&mut f.context, &[cancel_ix], &[&maker]).await;

        let bids = vec![];
        // Place n_orders new asks.
        let asks: Vec<CondensedOrder> = (0..n_orders)
            .map(|i| CondensedOrder {
                price_in_ticks: base_tick + i as u64 * 10,
                size_in_base_lots: 10,
            })
            .collect();

        let num_orders_placed = (bids.len() + asks.len()) as u64;

        let place_ix = create_new_multiple_order_instruction(
            &f.market,
            &maker.pubkey(),
            &f.base_mint,
            &f.quote_mint,
            &MultipleOrderPacket {
                bids,
                asks,
                client_order_id: None,
                reject_post_only: true,
            },
        );

        let place_cu = send_tx_measure_cu(&mut f.context, &[place_ix], &[&maker]).await;

        let round_cu = cancel_cu + place_cu;

        let cancel_round_cu = cancel_cu;
        let cancel_per = cancel_round_cu / num_existing_orders;
        total_cancel_cu += cancel_round_cu;
        total_num_cancels += num_existing_orders;

        let place_round_cu = place_cu;
        let place_per = place_round_cu / num_orders_placed;
        total_place_cu += place_round_cu;
        total_num_places += num_orders_placed;

        writeln!(
            &mut f.logs,
            "  Round {} (cancel {:>3} + place {n_orders:>3})   {:>6} CU  (avg cancel: {:>4}, avg place: {:>4})",
            round + 1,
            num_existing_orders,
            round_cu,
            cancel_per,
            place_per,
        )?;

        num_existing_orders = num_orders_placed;
    }

    writeln!(
        &mut f.logs,
        "  Average cancel  ({n_rounds} rounds)       {:>6} CU",
        total_cancel_cu / total_num_cancels,
    )?;
    writeln!(
        &mut f.logs,
        "  Average place                    {:>6} CU",
        total_place_cu / total_num_places,
    )?;

    writeln!(&mut f.logs, "\n{}", "=".repeat(60))?;

    Ok(())
}
