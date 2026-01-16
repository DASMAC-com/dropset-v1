use anyhow::anyhow;

use crate::oanda::{
    CurrencyPair,
    OandaCandlestickResponse,
};

pub struct MakerContext {
    /// The currency pair.
    pair: CurrencyPair,
    /// The mid price as quote / base.
    mid_price: f64,
    /// The total size of bids filled in base atoms.
    bid_fills: u64,
    /// The total size of asks filled in base atoms.
    ask_fills: u64,
}

impl MakerContext {
    pub fn mid_price(&self) -> f64 {
        self.mid_price
    }

    pub fn base_inventory(&self) -> i128 {
        self.bid_fills as i128 - self.ask_fills as i128
    }

    pub fn update_price_from_candlestick(
        &mut self,
        candlestick_response: OandaCandlestickResponse,
    ) -> anyhow::Result<()> {
        let maker_pair = self.pair.to_string();
        let response_pair = candlestick_response.instrument;
        if maker_pair != response_pair {
            anyhow::bail!("Maker and and candlestick response pair don't match. {maker_pair} != {response_pair}");
        }

        if !candlestick_response.candles.is_sorted_by_key(|c| c.time) {
            anyhow::bail!("Candlesticks aren't sorted by time (ascending).");
        }

        let latest = candlestick_response.candles.last();
        let latest_price = match latest {
            Some(candlestick) => {
                candlestick
                    .mid
                    .as_ref()
                    .ok_or_else(|| {
                        let err = anyhow::anyhow!("`mid` price not found in the last candlestick.");
                        err
                    })?
                    .c
            }
            None => anyhow::bail!("There are zero candlesticks in the candlestick response"),
        };

        self.mid_price = latest_price;

        Ok(())
    }
}
