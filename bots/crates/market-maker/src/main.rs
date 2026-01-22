//! Creates a market making bot that utilizes the strategy defined in [`crate::calculate_spreads`].

use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    time::Duration,
};

use crate::{
    cli::initialize_context_from_cli,
    maker_context::MakerContext,
    oanda::{
        query_price_feed,
        CandlestickGranularity,
        OandaArgs,
    },
};

mod program_subscribe;

use client::{
    print_kv,
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use program_subscribe::program_subscribe;
use strum_macros::Display;
use tokio::{
    sync::watch,
    time::sleep,
};

pub mod calculate_spreads;
pub mod maker_context;
pub mod oanda;

pub mod cli;
pub mod load_env;

const WS_URL: &str = "ws://localhost:8900";
pub const GRANULARITY: CandlestickGranularity = CandlestickGranularity::M15;
pub const NUM_CANDLES: u64 = 1;
const THROTTLE_WINDOW_MS: u64 = 500;

#[derive(Debug, Copy, Clone, Display)]
pub enum Update {
    MakerState,
    Price,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the maker context from the cli args.
    let reqwest_client = reqwest::Client::new();
    let rpc = CustomRpcClient::new(
        None,
        Some(SendTransactionConfig {
            compute_budget: Some(2000000),
            debug_logs: Some(true),
            program_id_filter: HashSet::from([dropset_interface::program::ID]),
        }),
    );
    let ctx = initialize_context_from_cli(&rpc, &reqwest_client).await?;

    let pair = ctx.pair;
    let maker_ctx = Rc::new(RefCell::new(ctx));

    // Create the sender/receiver to facilitate notifications of mutations from the program
    // subscription and price feed poller tasks.
    let (sender, receiver) = watch::channel(Update::MakerState);

    let oanda_args = OandaArgs {
        auth_token: load_env::oanda_auth_token(),
        pair,
        granularity: GRANULARITY,
        num_candles: NUM_CANDLES,
    };

    tokio::select! {
        r1 = program_subscribe(maker_ctx.clone(), sender.clone(), WS_URL) => {
            println!("Program subscription errored out: {r1:#?}");
        },
        r2 = poll_price_feed(maker_ctx.clone(), sender.clone(), reqwest_client, oanda_args) => {
            println!("Price feed poller errored out: {r2:#?}");
        },
        r3 = throttled_order_update(maker_ctx.clone(), receiver, &rpc, THROTTLE_WINDOW_MS) => {
            println!("Throttled order update errored out: {r3:#?}");
        }
    }

    Ok(())
}

async fn throttled_order_update(
    maker_ctx: Rc<RefCell<MakerContext>>,
    mut rx: watch::Receiver<Update>,
    rpc: &CustomRpcClient,
    throttle_window_ms: u64,
) -> anyhow::Result<()> {
    loop {
        // Wait until the value has changed. Not equality wise, but a sender posting a new value.
        rx.changed().await?;

        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
        let msg = format!("[{timestamp}]");
        print_kv!(msg, *rx.borrow());

        // Then cancel all orders and post new ones.
        let (maker_keypair, instructions) = {
            let ctx = maker_ctx.try_borrow()?;
            let maker_keypair = ctx.keypair.insecure_clone();
            let instructions = ctx.create_cancel_and_post_instructions()?;
            (maker_keypair, instructions)
        };

        rpc.send_and_confirm_txn(&maker_keypair, &[&maker_keypair], &instructions)
            .await?;

        // Sleep for the throttle window in milliseconds before doing work again.
        // This effectively means the loop only does the cancel/post work once every window of time.
        sleep(Duration::from_millis(throttle_window_ms)).await;
    }
}

async fn poll_price_feed(
    maker_ctx: Rc<RefCell<MakerContext>>,
    sender: watch::Sender<Update>,
    client: reqwest::Client,
    oanda_args: OandaArgs,
) -> anyhow::Result<()> {
    const POLL_INTERVAL_MS: u64 = 5000;
    let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));

    loop {
        interval.tick().await;

        match query_price_feed(&oanda_args, &client).await {
            Ok(response) => {
                // Update the price in the maker context and then notify with `watch::Sender` that
                // the context has updated.
                maker_ctx
                    .try_borrow_mut()?
                    .update_price_from_candlestick(response)?;
                sender.send(Update::Price)?;
                print_kv!("New mid price", maker_ctx.try_borrow()?.mid_price());
            }
            Err(e) => eprintln!("Price feed error: {e:#?}"),
        }
    }
}
