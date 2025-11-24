//! See [`main`].

use std::collections::HashMap;

use dropset_interface::{
    seeds::event_authority,
    state::market_header::MARKET_ACCOUNT_DISCRIMINANT,
};
use futures::StreamExt;
use grpc_stream::parse_update::{
    parse_update,
    InstructionEventsWithIndices,
    ParsedUpdate,
};
use solana_sdk::pubkey::Pubkey;
use tokio::time::{
    sleep,
    Duration,
};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::{
    geyser::{
        subscribe_request_filter_accounts_filter::Filter,
        subscribe_request_filter_accounts_filter_memcmp::Data,
    },
    prelude::*,
};

/// An example for streaming and parsing `dropset` events from an active, local GRPC stream on
/// a `geyser`-enabled client.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let endpoint = "http://localhost:10000";

    let mut client = GeyserGrpcClient::build_from_static(endpoint)
        .connect()
        .await?;

    let mut stream = client
        .subscribe_once(SubscribeRequest {
            accounts: HashMap::from([(
                "owned market account PDA data".to_string(),
                SubscribeRequestFilterAccounts {
                    account: vec![],
                    owner: vec![Pubkey::new_from_array(dropset_interface::program::ID).to_string()],
                    filters: vec![SubscribeRequestFilterAccountsFilter {
                        filter: Some(Filter::Memcmp(SubscribeRequestFilterAccountsFilterMemcmp {
                            offset: 0,
                            data: Some(Data::Bytes(
                                MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes().to_vec(),
                            )),
                        })),
                    }],
                    nonempty_txn_signature: Some(true),
                },
            )]),
            slots: HashMap::new(),
            transactions: HashMap::from([(
                "event authority pda instruction data".to_string(),
                SubscribeRequestFilterTransactions {
                    failed: None,
                    signature: None,
                    vote: None,
                    account_exclude: vec![],
                    account_include: vec![],
                    account_required: vec![Pubkey::new_from_array(event_authority::ID).to_string()],
                },
            )]),
            transactions_status: HashMap::new(),
            blocks: HashMap::new(),
            entry: HashMap::new(),
            blocks_meta: HashMap::new(),
            commitment: Some(CommitmentLevel::Processed.into()),
            accounts_data_slice: vec![],
            ping: None,
            from_slot: None,
        })
        .await?;

    while let Some(message) = stream.next().await {
        match message {
            Ok(msg) => {
                if let Some(update) = msg.update_oneof {
                    let update = parse_update(update);

                    match update {
                        Some(ParsedUpdate::Market(market)) => {
                            println!("{:?}", market);
                        }
                        Some(ParsedUpdate::EmittedEvents { logs, events }) => {
                            if !logs.is_empty() {
                                for log in logs.iter().filter(|s| s.contains("[DEBUG]: ")) {
                                    println!("------ LOGS -------");
                                    println!("{:?}", log);
                                }
                            }
                            for inner_ixn_with_events in events {
                                let InstructionEventsWithIndices {
                                    parent_index,
                                    inner_index: _,
                                    events,
                                } = inner_ixn_with_events;
                                if !events.is_empty() {
                                    println!("----- EVENTS ------");
                                    println!("Parent index: {}", parent_index);
                                    println!("{:?}", events);
                                }
                            }
                        }
                        None => {}
                    }
                }
            }
            Err(error) => {
                eprintln!("❌ Stream error: {}", error);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    Ok(())
}
