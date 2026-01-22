use clap::{
    command,
    Parser,
};
use client::transactions::CustomRpcClient;
use solana_address::Address;

use crate::{
    load_env::{
        self,
        oanda_auth_token,
    },
    maker_context::MakerContext,
    oanda::{
        query_price_feed,
        CurrencyPair,
        OandaArgs,
    },
    GRANULARITY,
    NUM_CANDLES,
};

#[derive(Parser)]
#[command(name = "market-maker")]
pub struct CliArgs {
    /// Base mint address.
    #[arg(short = 'b', long)]
    pub base_mint: Address,

    /// Quote mint address.
    #[arg(short = 'q', long)]
    pub quote_mint: Address,

    /// The [`CurrencyPair`] as a string. The format is `{BASE}_{QUOTE}`; e.g. `EUR_USD`.
    #[arg(short = 'p', long)]
    pub pair: CurrencyPair,

    /// The target base inventory in atoms that the model implementation will gravitate towards.
    /// This value is absolute, meaning a passed value of zero when the maker has existing base
    /// already will result in the maker immediately placing aggressive asks and passive/wide bids.
    #[arg(long)]
    pub target_base: u64,
}

/// Loads the maker context from passed CLI arguments and a few expected environment variables.
/// See [`crate::load_env`] for the expected environment variables.
pub async fn initialize_context_from_cli(
    rpc: &CustomRpcClient,
    reqwest_client: &reqwest::Client,
) -> anyhow::Result<MakerContext> {
    let CliArgs {
        base_mint,
        quote_mint,
        pair,
        target_base,
    } = CliArgs::parse();

    let maker = load_env::maker_keypair().insecure_clone();

    let initial_price_feed_response = query_price_feed(
        &OandaArgs {
            auth_token: oanda_auth_token(),
            pair,
            granularity: GRANULARITY,
            num_candles: NUM_CANDLES,
        },
        reqwest_client,
    )
    .await?;

    MakerContext::init(
        rpc,
        maker,
        base_mint,
        quote_mint,
        pair,
        target_base,
        initial_price_feed_response,
    )
}
