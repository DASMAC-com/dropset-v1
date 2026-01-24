//! Creates a market making bot that utilizes the strategy defined in [`crate::calculate_spreads`].

use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use client::{
    print_kv,
    transactions::{
        CustomRpcClient,
        SendTransactionConfig,
    },
};
use dropset_interface::state::market_header::MARKET_ACCOUNT_DISCRIMINANT;
use solana_address::Address;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{
        CommitmentConfig,
        RpcAccountInfoConfig,
        RpcProgramAccountsConfig,
    },
    rpc_filter::{
        Memcmp,
        RpcFilterType,
    },
};
use strum_macros::Display;
use tokio::{
    sync::watch,
    time::sleep,
};
use tokio_stream::StreamExt;
use transaction_parser::views::try_market_view_all_from_owner_and_data;

use crate::{
    cli::initialize_context_from_cli,
    maker_context::MakerContext,
    oanda::{
        query_price_feed,
        CandlestickGranularity,
        OandaArgs,
    },
};

pub mod calculate_spreads;
pub mod maker_context;
pub mod model_parameters;
pub mod oanda;

pub mod cli;
pub mod load_env;

const WS_URL: &str = "ws://localhost:8900";
pub const GRANULARITY: CandlestickGranularity = CandlestickGranularity::M15;
pub const NUM_CANDLES: u64 = 1;
const THROTTLE_WINDOW_MS: u64 = 500;

#[derive(Debug, Copy, Clone, Display)]
pub enum TaskUpdate {
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
    let (sender, receiver) = watch::channel(TaskUpdate::MakerState);

    let oanda_args = OandaArgs {
        auth_token: load_env::oanda_auth_token(),
        pair,
        granularity: GRANULARITY,
        num_candles: NUM_CANDLES,
    };

    tokio::select! {
        r1 = program_subscribe(maker_ctx.clone(), sender.clone(), WS_URL) => {
            println!("Program subscription terminated: {r1:#?}");
        },
        r2 = poll_price_feed(maker_ctx.clone(), sender.clone(), reqwest_client, oanda_args) => {
            println!("Price feed poll loop terminated: {r2:#?}");
        },
        r3 = throttled_order_update(maker_ctx.clone(), receiver, &rpc, THROTTLE_WINDOW_MS) => {
            println!("Throttled order update loop terminated: {r3:#?}");
        }
    }

    Ok(())
}

/// The indefinite task loop for the event-driven program subscription.
///
/// It updates the maker state any time the market account state changes per the RPC client's
/// websocket subcription and subsequently notifies the [`throttled_order_update`] task of a
/// [`TaskUpdate::MakerState`] update.
pub async fn program_subscribe(
    maker_ctx: Rc<RefCell<MakerContext>>,
    sender: watch::Sender<TaskUpdate>,
    ws_url: &str,
) -> anyhow::Result<()> {
    // The market address should never change, so store it once for filtering later.
    let market_address = maker_ctx.try_borrow()?.market_ctx.market.to_string();
    let ws_client = PubsubClient::new(ws_url).await?;

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes().to_vec(),
        ))]),
        account_config: RpcAccountInfoConfig {
            commitment: Some(CommitmentConfig::confirmed()),
            encoding: Some(solana_client::rpc_config::UiAccountEncoding::Base64),
            data_slice: None,
            min_context_slot: None,
        },
        with_context: Some(true),
        sort_results: Some(true),
    };

    let (mut stream, _) = ws_client
        .program_subscribe(&dropset_interface::program::ID, Some(config))
        .await
        .context("Couldn't subscribe to program")?;

    while let Some(account) = stream.next().await {
        if account.value.pubkey != market_address {
            continue;
        }
        let owner = Address::from_str(account.value.account.owner.as_str())
            .expect("Should be a valid address");
        let account_data = account
            .value
            .account
            .data
            .decode()
            .expect("Should decode account data");
        let market_view = try_market_view_all_from_owner_and_data(owner, &account_data)
            .expect("Should convert to a valid market account's data");

        // Update the maker state in the maker context.
        maker_ctx
            .try_borrow_mut()?
            .update_maker_state(market_view)?;

        // And notify the `watch::Receiver` of a new maker state update.
        sender.send(TaskUpdate::MakerState)?;
    }

    Ok(())
}

/// Oanda recommends limiting to twice per second (500ms interval). Thus anything other greater
/// than 500ms here should be fine.
///
/// See: <https://developer.oanda.com/rest-live-v20/best-practices/>
const POLL_INTERVAL_MS: u64 = 5000;

/// The indefinite task loop for polling the price feed endpoint.
///
/// On each loop iteration, it updates the maker context price info and notifies the
/// [`throttled_order_update`] task of a [`TaskUpdate::Price`] update.
async fn poll_price_feed(
    maker_ctx: Rc<RefCell<MakerContext>>,
    sender: watch::Sender<TaskUpdate>,
    client: reqwest::Client,
    oanda_args: OandaArgs,
) -> anyhow::Result<()> {
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
                sender.send(TaskUpdate::Price)?;
                print_kv!("New mid price", maker_ctx.try_borrow()?.mid_price());
            }
            Err(e) => eprintln!("Price feed error: {e:#?}"),
        }
    }
}

/// The indefinite task loop to update orders whenever the [`watch::Receiver`] receives a message
/// from another task that indicates a [`TaskUpdate`] has occurred. Order submissions are
/// throttled so that they're updated at most one time per interval window.
///
/// It cancels old orders and posts new orders whenever the maker's orders would change due to a new
/// price from the price feed response or new market state.
async fn throttled_order_update(
    maker_ctx: Rc<RefCell<MakerContext>>,
    mut rx: watch::Receiver<TaskUpdate>,
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

        if !instructions.is_empty() {
            rpc.send_and_confirm_txn(&maker_keypair, &[&maker_keypair], &instructions)
                .await?;
        }

        // Sleep for the throttle window in milliseconds before doing work again.
        // This effectively means the loop only does the cancel/post work once every window of time.
        sleep(Duration::from_millis(throttle_window_ms)).await;
    }
}
