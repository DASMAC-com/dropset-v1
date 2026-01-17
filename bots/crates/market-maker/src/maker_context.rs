use client::{
    context::market::MarketContext,
    transactions::CustomRpcClient,
};
use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::{
        sector::SectorIndex,
        user_order_sectors::OrderSectors,
    },
};
use itertools::Itertools;
use price::to_order_info_args;
use solana_address::Address;
use solana_sdk::{
    message::Instruction,
    signature::Keypair,
};
use transaction_parser::views::{
    MarketSeatView,
    MarketViewAll,
    OrderView,
};

use crate::{
    calculate_spreads::{
        half_spread,
        reservation_price,
    },
    oanda::{
        CurrencyPair,
        OandaCandlestickResponse,
    },
};

const ORDER_SIZE: u64 = 10_000;

pub struct MakerState {
    pub transaction_version: u64,
    pub address: Address,
    pub seat: MarketSeatView,
    pub bids: Vec<OrderView>,
    pub asks: Vec<OrderView>,
    pub base_inventory: u64,
    pub quote_inventory: u64,
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
    pub fn new_from_market(
        transaction_version: u64,
        maker_address: Address,
        market: &MarketViewAll,
    ) -> anyhow::Result<Self> {
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

        // Sum the maker's base inventory by adding the seat balance + the bid collateral amounts.
        let base_inventory = bids
            .iter()
            .fold(seat.base_available, |acc, seat| acc + seat.base_remaining);

        // Sum the maker's quote inventory by adding the seat balance + the ask collateral amounts.
        let quote_inventory = asks
            .iter()
            .fold(seat.quote_available, |acc, seat| acc + seat.quote_remaining);

        Ok(Self {
            transaction_version,
            address: maker_address,
            seat,
            bids,
            asks,
            base_inventory,
            quote_inventory,
        })
    }
}

pub struct MakerContext<'a> {
    /// The maker's keypair.
    keypair: Keypair,
    market_ctx: &'a MarketContext,
    /// The maker's address.
    address: Address,
    /// The currency pair.
    pair: CurrencyPair,
    /// The maker's initial state.
    initial_state: MakerState,
    /// The maker's latest state.
    latest_state: MakerState,
    /// The change in the market maker's base inventory value as a signed integer.
    ///
    /// In the A-S model `q` represents the base inventory as a reflection of the maker's net short
    /// (negative) or long (positive) position. The change in base inventory from initial to
    /// current state thus can be used in place of `q` to achieve the effect of always returning to
    /// the initial base inventory amount.
    ///
    /// When `q` is negative, the maker is below the desired/target inventory amount, and when `q`
    /// is positive, the maker is above the desired/target inventory amount.
    ///
    /// In practice, this has two opposing effects.
    /// - When q is negative, it pushes the spread upwards so that bid prices are closer to the
    ///   [`crate::calculate_spreads::reservation_price`] and ask prices are further away. This
    ///   effectively increases the likelihood of getting bids filled and vice versa for asks.
    /// - When q is positive, it pushes the spread downwards so that ask prices are closer to the
    ///   [`crate::calculate_spreads::reservation_price`] price and bid prices are further away.
    ///   This effectively increases the likelihood of getting asks filled and vice versa for bids.
    pub base_inventory_delta: i128,
    /// The change in quote inventory since the initial maker state was created.
    /// This isn't used by the A-S model but is helpful for debugging purposes.
    pub quote_inventory_delta: i128,
    /// The reference mid price, expressed as quote per 1 base.
    ///
    /// In the A–S model this is an exogenous “fair price” process; in practice you can source it
    /// externally (e.g. FX feed) or derive it internally from the venue’s top-of-book.
    /// It anchors the reservation price and thus the bid/ask quotes via the spread model.
    mid_price: f64,
    /// The transaction version of the last successful cancel + order transaction.
    last_successful_txn_version: Option<u64>,
}

impl MakerContext<'_> {
    /// See [`MakerContext::mid_price`].
    pub fn mid_price(&self) -> f64 {
        self.mid_price
    }

    /// Checks if the latest state is definitively stale by comparing the transaction version in the
    /// latest state to the transaction version of the last submitted cancel + post transaction.
    ///
    /// NOTE: A value of `false` doesn't mean the latest state is definitively not stale, because
    /// the maker's orders can get filled at any time.
    pub fn is_latest_state_definitely_stale(&self) -> bool {
        self.latest_state.transaction_version < self.last_successful_txn_version.unwrap_or(0)
    }

    pub fn maker_seat(&self) -> SectorIndex {
        self.latest_state.seat.index
    }

    pub async fn cancel_all_and_post_new(&mut self, rpc: &CustomRpcClient) -> anyhow::Result<()> {
        // NOTE: The bids and asks here might be stale due to fills. This will cause the cancel
        // order attempt to fail. This is an expected possible error.
        let cancel_bid_instructions = self
            .latest_state
            .bids
            .iter()
            .map(|bid| {
                self.market_ctx.cancel_order(
                    self.address,
                    CancelOrderInstructionData::new(bid.encoded_price, true, self.maker_seat()),
                )
            })
            .collect_vec();
        let cancel_ask_instructions = self
            .latest_state
            .asks
            .iter()
            .map(|ask| {
                self.market_ctx.cancel_order(
                    self.address,
                    CancelOrderInstructionData::new(ask.encoded_price, false, self.maker_seat()),
                )
            })
            .collect_vec();

        let (bid_price, ask_price) = self.get_bid_and_ask_prices();
        let to_post_ixn = |price: f64, size: u64, is_bid: bool, seat_index: SectorIndex| {
            to_order_info_args(price, size)
                .map_err(|e| anyhow::anyhow! {"{e:#?}"})
                .map(|args| {
                    PostOrderInstructionData::new(
                        args.0, args.1, args.2, args.3, is_bid, seat_index,
                    )
                })
        };

        let post_instructions = vec![
            self.market_ctx.post_order(
                self.address,
                to_post_ixn(bid_price, ORDER_SIZE, true, self.maker_seat())?,
            ),
            self.market_ctx.post_order(
                self.address,
                to_post_ixn(ask_price, ORDER_SIZE, false, self.maker_seat())?,
            ),
        ];

        let ixns = [
            cancel_ask_instructions,
            cancel_bid_instructions,
            post_instructions,
        ]
        .into_iter()
        .concat();

        rpc.send_and_confirm_txn(
            &self.keypair,
            &[&self.keypair],
            ixns.into_iter()
                .map(Instruction::from)
                .collect_vec()
                .as_ref(),
        )
        .await?;

        Ok(())
    }

    pub fn update_state_and_inventory_deltas(
        &mut self,
        transaction_version: u64,
        new_market_state: &MarketViewAll,
    ) -> anyhow::Result<()> {
        self.latest_state =
            MakerState::new_from_market(transaction_version, self.address, new_market_state)?;
        self.base_inventory_delta =
            self.latest_state.base_inventory as i128 - self.initial_state.base_inventory as i128;
        self.quote_inventory_delta =
            self.latest_state.quote_inventory as i128 - self.initial_state.quote_inventory as i128;

        Ok(())
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

    /// Calculates the model's output bid and ask prices as a function of the current mid price and
    /// the maker's base inventory delta.

    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    // These units need to be properly normalized/scaled to atoms (or not).
    fn get_bid_and_ask_prices(&self, base_decimals: i32, quote_decimals: i32) -> (f64, f64) {
        let normalization_factor = 10f64.powi(quote_decimals - base_decimals);
        let normalize = |price: f64| price * normalization_factor;

        let q_atoms = self.base_inventory_delta;
        let scale = 10i128.pow(base_decimals as u32); // base_decimals <= 18-ish is safe
        let q_base_units: f64 =
            (q_atoms / scale) as f64 + (q_atoms % scale) as f64 / (scale as f64);

        let reservation_price = reservation_price(self.mid_price(), q_base_units);
        let bid_price = reservation_price - half_spread();
        let ask_price = reservation_price + half_spread();

        (normalize(bid_price), normalize(ask_price))
    }
}
