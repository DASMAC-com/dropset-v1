use std::{
    fmt::Display,
    str::FromStr,
};

use chrono::{
    DateTime,
    Utc,
};
use rust_decimal::Decimal;
use serde::{
    Deserialize,
    Deserializer,
};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrencyPair {
    pub base: Currency,
    pub quote: Currency,
}

impl Display for CurrencyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (base, quote) = (self.base, self.quote);
        write!(f, "{base}_{quote}")
    }
}

impl FromStr for CurrencyPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, quote) = s
            .split_once('_')
            .ok_or_else(|| anyhow::anyhow!("Invalid currency pair format: {s}"))?;

        Ok(CurrencyPair {
            base: base.parse()?,
            quote: quote.parse()?,
        })
    }
}

impl<'de> Deserialize<'de> for CurrencyPair {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#CandlestickResponse>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OandaCandlestickResponse {
    pub instrument: CurrencyPair,
    pub granularity: CandlestickGranularity,
    pub candles: Vec<OandaCandlestick>,
}

/// See: <https://developer.oanda.com/rest-live-v20/instrument-df/#Candlestick>
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OandaCandlestickData {
    #[serde(deserialize_with = "rust_decimal::serde::arbitrary_precision::deserialize")]
    pub o: Decimal,
    #[serde(deserialize_with = "rust_decimal::serde::arbitrary_precision::deserialize")]
    pub h: Decimal,
    #[serde(deserialize_with = "rust_decimal::serde::arbitrary_precision::deserialize")]
    pub l: Decimal,
    #[serde(deserialize_with = "rust_decimal::serde::arbitrary_precision::deserialize")]
    pub c: Decimal,
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::BufReader,
        path::PathBuf,
    };

    use chrono::{
        DateTime,
        Utc,
    };

    use crate::oanda::{
        CandlestickGranularity,
        Currency,
        CurrencyPair,
        OandaCandlestick,
        OandaCandlestickData,
        OandaCandlestickResponse,
    };

    #[test]
    fn deserialize_candlestick_decimals() {
        let path = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
            .join("bots/crates/market-maker/test_oanda_response.json");
        let file = File::open(path).expect("Should find test oanda response json file");
        let reader = BufReader::new(file);
        let res: OandaCandlestickResponse =
            serde_json::from_reader(reader).expect("Should parse the response json");

        let parse_utc = |s: &str| -> DateTime<Utc> {
            DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
        };

        let expected = OandaCandlestickResponse {
            instrument: CurrencyPair {
                base: Currency::EUR,
                quote: Currency::USD,
            },
            granularity: CandlestickGranularity::M15,
            candles: vec![
                OandaCandlestick {
                    complete: true,
                    volume: 236,
                    time: parse_utc("2026-01-19T19:45:00.000000000Z"),
                    mid: Some(OandaCandlestickData {
                        o: rust_decimal::dec!(1.16439),
                        h: rust_decimal::dec!(1.16444),
                        l: rust_decimal::dec!(1.16426),
                        c: rust_decimal::dec!(1.16433),
                    }),
                    bid: None,
                    ask: None,
                },
                OandaCandlestick {
                    complete: false,
                    volume: 75,
                    time: parse_utc("2026-01-19T20:00:00.000000000Z"),
                    mid: Some(OandaCandlestickData {
                        o: rust_decimal::dec!(1.16432),
                        h: rust_decimal::dec!(1.16446),
                        l: rust_decimal::dec!(1.16432),
                        c: rust_decimal::dec!(1.16440),
                    }),
                    bid: None,
                    ask: None,
                },
            ],
        };

        assert_eq!(res, expected);
    }
}
