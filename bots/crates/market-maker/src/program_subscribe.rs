use std::{
    cell::RefCell,
    rc::Rc,
    str::FromStr,
};

use anyhow::Context;
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
use tokio_stream::StreamExt;
use transaction_parser::views::try_market_view_all_from_owner_and_data;

use crate::maker_context::MakerContext;

pub async fn program_subscribe(
    maker_ctx: Rc<RefCell<MakerContext>>,
    ws_url: &str,
) -> anyhow::Result<()> {
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

        // For now debug with print statement, eventually, this will mutate the MakerContext
        // state and update it.
        println!("new maker state\n{market_view:#?}");
        maker_ctx
            .try_borrow_mut()?
            .update_maker_state(&market_view)?;
    }

    Ok(())
}
