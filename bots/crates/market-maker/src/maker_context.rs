use dropset_interface::state::user_order_sectors::OrderSectors;
use itertools::Itertools;
use solana_address::Address;
use transaction_parser::views::{
    MarketSeatView,
    MarketViewAll,
    OrderView,
};

use crate::oanda::{
    CurrencyPair,
    OandaCandlestickResponse,
};

pub struct MakerState {
    address: Address,
    seat: MarketSeatView,
    bids: Vec<OrderView>,
    asks: Vec<OrderView>,
}

fn find_maker_seat(market: &MarketViewAll, maker: &Address) -> anyhow::Result<MarketSeatView> {
    let res = market.seats.binary_search_by_key(maker, |v| v.user);
    let seat = match res {
        Ok(found_index) => market
            .seats
            .get(found_index)
            .expect("Seat index should be valid")
            .clone(),
        Err(_insert_index) => anyhow::bail!("Couldn't find maker in seat list."),
    };

    Ok(seat)
}

impl MakerState {
    pub fn new_from_market(maker_address: Address, market: &MarketViewAll) -> anyhow::Result<Self> {
        let seat = find_maker_seat(market, &maker_address)?;

        // Convert a user's order sectors into a Vec<u32> of prices.
        let to_prices = |order_sectors: &OrderSectors| -> Vec<u32> {
            order_sectors
                .iter()
                .filter(|b| !b.is_free())
                .map(|p| u32::from_le_bytes(p.encoded_price.as_array()))
                .collect_vec()
        };

        let bid_prices = to_prices(&seat.user_order_sectors.bids);
        let ask_prices = to_prices(&seat.user_order_sectors.asks);

        // Given a price and a collection of orders, find the unique order associated with the pric
        // passed. This is just for the bids and asks in this local function so all passed prices
        // should map to a valid order, hence the `.expect(...)` calls instead of returning Results.
        let find_order_by_price = |price: &u32, orders: &[OrderView]| {
            let order_list_index = orders
                .binary_search_by_key(price, |order| order.encoded_price)
                .expect("Should find order with matching encoded price");
            orders
                .get(order_list_index)
                .expect("Index should correspond to a valid order")
                .clone()
        };

        // Map each bid price to its corresponding order.
        let bids = bid_prices
            .iter()
            .map(|price| find_order_by_price(price, &market.bids))
            .collect_vec();

        // Map each ask price to its corresponding order.
        let asks = ask_prices
            .iter()
            .map(|price| find_order_by_price(price, &market.asks))
            .collect_vec();

        Ok(Self {
            address: maker_address,
            seat,
            bids,
            asks,
        })
    }
}

pub struct MakerContext {
    /// The currency pair.
    pair: CurrencyPair,
    /// The maker's initial seat state.
    initial_state: MarketSeatView,
    /// The maker's latest seat state
    latest_state: MarketSeatView,
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
