use crate::oanda::{
    CandlestickGranularity,
    CurrencyPair,
    OandaCandlestickResponse,
};

const OANDA_BASE_URL: &str = "https://api-fxpractice.oanda.com/v3";

pub async fn query_price_feed(
    auth_token: &str,
    pair: CurrencyPair,
    granularity: CandlestickGranularity,
    num_candles: u64,
    client: reqwest::Client,
) -> anyhow::Result<OandaCandlestickResponse> {
    let url =
        format!("{OANDA_BASE_URL}/{pair}/candles?count={num_candles}&granularity={granularity}");
    let response = client.get(url).bearer_auth(auth_token).send().await?;
    let text = response.text().await?;

    serde_json::from_str(text.as_str()).map_err(|e| e.into())
}
