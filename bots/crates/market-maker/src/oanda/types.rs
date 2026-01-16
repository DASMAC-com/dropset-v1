use std::fmt::Display;

use chrono::{
    DateTime,
    Utc,
};
use serde::Deserialize;
use strum_macros::{
    AsRefStr,
    Display,
    EnumString,
};

/// Oanda's Majors currencies. All variants are ISO 4217 currencies.
///
/// See: <https://www.oanda.com/currency-converter/en/currencies/>
/// See: <https://en.wikipedia.org/wiki/ISO_4217>
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, EnumString, AsRefStr, Display)]
pub enum Currency {
    /// United Arab Emirates Dirham
    AED,
    /// Australian Dollar
    AUD,
    /// Brazilian Real
    BRL,
    /// Canadian Dollar
    CAD,
    /// Swiss Franc
    CHF,
    /// Chinese Yuan Renminbi
    CNY,
    /// Euro
    EUR,
    /// British Pound
    GBP,
    /// Hong Kong Dollar
    HKD,
    /// Indian Rupee
    INR,
    /// Japanese Yen
    JPY,
    /// Mexican Peso
    MXN,
    /// Malaysian Ringgit
    MYR,
    /// Philippine Peso
    PHP,
    /// Saudi Riyal
    SAR,
    /// Swedish Krona
    SEK,
    /// Singapore Dollar
    SGD,
    /// Thai Baht
    THB,
    /// US Dollar
    USD,
    /// South African Rand
    ZAR,
}

/// OANDA candlestick time-bucket sizes and their alignment rules (minute/hour/day/week/month).
/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#CandlestickGranularity>
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, EnumString, AsRefStr, Display)]
pub enum CandlestickGranularity {
    /// 5 second candlesticks, minute alignment
    S5,
    /// 10 second candlesticks, minute alignment
    S10,
    /// 15 second candlesticks, minute alignment
    S15,
    /// 30 second candlesticks, minute alignment
    S30,

    /// 1 minute candlesticks, minute alignment
    M1,
    /// 2 minute candlesticks, hour alignment
    M2,
    /// 4 minute candlesticks, hour alignment
    M4,
    /// 5 minute candlesticks, hour alignment
    M5,
    /// 10 minute candlesticks, hour alignment
    M10,
    /// 15 minute candlesticks, hour alignment
    M15,
    /// 30 minute candlesticks, hour alignment
    M30,

    /// 1 hour candlesticks, hour alignment
    H1,
    /// 2 hour candlesticks, day alignment
    H2,
    /// 3 hour candlesticks, day alignment
    H3,
    /// 4 hour candlesticks, day alignment
    H4,
    /// 6 hour candlesticks, day alignment
    H6,
    /// 8 hour candlesticks, day alignment
    H8,
    /// 12 hour candlesticks, day alignment
    H12,

    /// 1 day candlesticks, day alignment
    D,
    /// 1 week candlesticks, aligned to start of week
    W,
    /// 1 month candlesticks, aligned to first day of the month
    M,
}

#[derive(Debug, Clone)]
pub struct CurrencyPair {
    pub base: Currency,
    pub quote: Currency,
}

impl Display for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (base, quote) = (self.base, self.quote);
        writeln!(f, "{base}_{quote}")
    }
}

/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#CandlestickResponse>
#[derive(Debug, Clone, Deserialize)]
pub struct OandaCandlestickResponse {
    pub instrument: String,
    pub granularity: CandlestickGranularity,
    pub candles: Vec<OandaCandlestick>,
}

/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#Candlestick>
#[derive(Debug, Clone, Deserialize)]
pub struct OandaCandlestick {
    /// The start time of the candlestick.
    pub time: DateTime<Utc>,
    /// The candlestick data based on bids. Only provided if bid-based candles were requested.
    pub bid: Option<OandaCandlestickData>,
    /// The candlestick data based on asks. Only provided if ask-based candles were requested.
    pub ask: Option<OandaCandlestickData>,
    /// The candlestick data based on midpoints. Only provided if midpoint-based candles were
    /// requested.
    pub mid: Option<OandaCandlestickData>,
    /// The number of prices created during the time-range represented by the candlestick.
    pub volume: u64,
    /// A flag indicating if the candlestick is complete. A complete candlestick is one whose
    /// ending time is not in the future.
    pub complete: bool,
}

/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#CandlestickData>
#[derive(Debug, Clone, Deserialize)]
pub struct OandaCandlestickData {
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
}
