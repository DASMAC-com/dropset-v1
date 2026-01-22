use std::collections::HashSet;

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
use price::{
    client_helpers::{
        decimal_pow10_i16,
        to_order_info_args,
    },
    to_order_info,
    OrderInfo,
};
use rust_decimal::Decimal;
use solana_address::Address;
use solana_keypair::Signer;
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

const ORDER_SIZE: u64 = 1_000;

#[derive(Debug)]
pub struct MakerState {
    pub address: Address,
    pub seat: MarketSeatView,
    pub bids: Vec<OrderView>,
    pub asks: Vec<OrderView>,
    /// The maker's current base inventory; i.e., the [`MarketSeatView::base_available`] + the
    /// base in all open orders.
    pub base_inventory: u64,
    /// The maker's current quote inventory; i.e., the [`MarketSeatView::quote_available`] + the
    /// quote in all open orders.
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

        // Sum the maker's base inventory by adding the seat balance + the bid collateral amounts.
        let base_inventory = bids
            .iter()
            .fold(seat.base_available, |acc, seat| acc + seat.base_remaining);

        // Sum the maker's quote inventory by adding the seat balance + the ask collateral amounts.
        let quote_inventory = asks
            .iter()
            .fold(seat.quote_available, |acc, seat| acc + seat.quote_remaining);

        Ok(Self {
            address: maker_address,
            seat,
            bids,
            asks,
            base_inventory,
            quote_inventory,
        })
    }
}

pub struct MakerContext {
    /// The maker's keypair.
    pub keypair: Keypair,
    pub market_ctx: MarketContext,
    /// The maker's address.
    pub maker_address: Address,
    /// The currency pair.
    pub pair: CurrencyPair,
    /// The maker's latest state.
    latest_state: MakerState,
    /// The target base amount in the maker's seat, in atoms.
    ///
    /// If the maker starts with 1,000 base atoms and the target base amount is 10,000, `q` will be
    /// equal to -9,000. This will indirectly influence the model to more aggressively place bids
    /// and thus return to a `q` value of zero.
    pub base_target_atoms: u64,
    /// The reference mid price, expressed as quote atom per 1 base atom.
    ///
    /// In the A–S model this is an exogenous “fair price” process; in practice you can source it
    /// externally (e.g. FX feed) or derive it internally from the venue’s top-of-book.
    /// It anchors the reservation price and thus the bid/ask quotes via the spread model.
    ///
    /// Note that the price as quote_atoms / base_atoms may differ from quote / base. Be sure to
    /// express the price as a ratio of atoms.
    mid_price: Decimal,
}

impl MakerContext {
    /// Creates a new maker context from a token pair.
    pub fn init(
        rpc: &CustomRpcClient,
        maker: Keypair,
        base_mint: Address,
        quote_mint: Address,
        pair: CurrencyPair,
        base_target_atoms: u64,
        initial_price_feed_response: OandaCandlestickResponse,
    ) -> anyhow::Result<Self> {
        let market_ctx =
            MarketContext::new_from_token_pair(rpc, base_mint, quote_mint, None, None)?;
        let market = market_ctx.view_market(rpc)?;
        let latest_state = MakerState::new_from_market(maker.pubkey(), &market)?;
        let mid_price = get_normalized_mid_price(initial_price_feed_response, &pair, &market_ctx)?;
        let maker_address = maker.pubkey();

        Ok(Self {
            keypair: maker,
            market_ctx,
            maker_address,
            pair,
            latest_state,
            base_target_atoms,
            mid_price,
        })
    }

    /// See [`MakerContext::mid_price`].
    pub fn mid_price(&self) -> Decimal {
        self.mid_price
    }

    pub fn maker_seat(&self) -> SectorIndex {
        self.latest_state.seat.index
    }

    /// In the A-S model `q` represents the base inventory as a reflection of the maker's net short
    /// (negative) or long (positive) position. The difference from the maker seat's current base
    /// to target base can thus be used as `q` to achieve the effect of always returning to the
    /// target base inventory amount.
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
    pub fn q(&self) -> Decimal {
        (Decimal::from(self.latest_state.base_inventory) - Decimal::from(self.base_target_atoms))
            / Decimal::from(10u64.pow(self.market_ctx.base.mint_decimals as u32))
    }

    pub fn create_cancel_and_post_instructions(&self) -> anyhow::Result<Vec<Instruction>> {
        // NOTE: The bids and asks here might be stale due to fills. This will cause the cancel
        // order attempt to fail. This is an expected possible error.
        let cancel_bid_instructions = self
            .latest_state
            .bids
            .iter()
            .map(|bid| {
                self.market_ctx.cancel_order(
                    self.maker_address,
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
                    self.maker_address,
                    CancelOrderInstructionData::new(ask.encoded_price, false, self.maker_seat()),
                )
            })
            .collect_vec();

        let (bid_price, ask_price) = self.get_bid_and_ask_prices();

        let to_post_ixn = |price: Decimal, size: u64, is_bid: bool, seat_index: SectorIndex| {
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
                self.maker_address,
                to_post_ixn(bid_price, ORDER_SIZE, true, self.maker_seat())?,
            ),
            self.market_ctx.post_order(
                self.maker_address,
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

        Ok(ixns.into_iter().map(Instruction::from).collect_vec())
    }

    pub fn update_maker_state(&mut self, new_market_state: &MarketViewAll) -> anyhow::Result<()> {
        self.latest_state = MakerState::new_from_market(self.maker_address, new_market_state)?;

        Ok(())
    }

    pub fn update_price_from_candlestick(
        &mut self,
        candlestick_response: OandaCandlestickResponse,
    ) -> anyhow::Result<()> {
        self.mid_price =
            get_normalized_mid_price(candlestick_response, &self.pair, &self.market_ctx)?;

        Ok(())
    }

    /// Calculates the model's output bid and ask prices based on the market's current mid price
    /// and the maker's current state.
    fn get_bid_and_ask_prices(&self) -> (Decimal, Decimal) {
        let reservation_price = reservation_price(self.mid_price(), self.q());
        let bid_price = reservation_price - half_spread();
        let ask_price = reservation_price + half_spread();

        (bid_price, ask_price)
    }
}

/// The maker essentially cancels all orders and then re-posts them. If any orders would be canceled
/// and then reposted in the same transaction, simply filter out the corresponding cancel + post
/// instruction for that order.
fn get_non_redundant_order_flow(
    latest_state: &MakerState,
    // Vec of (price, size) tuples.
    bid_posts: Vec<(Decimal, u64)>,
    // Vec of (price, size) tuples.
    ask_posts: Vec<(Decimal, u64)>,
) -> anyhow::Result<(
    Vec<CancelOrderInstructionData>,
    Vec<PostOrderInstructionData>,
)> {
    let post_bid_args = bid_posts
        .into_iter()
        .map(|(price, size)| {to_order_info_args(price, size).map_err(|e| anyhow::anyhow!("{e:#?}"))})
        .collect::<anyhow::Result<Vec<_>>>()?;

    let post_ask_args = ask_posts
        .into_iter()
        .map(|(price, size)| to_order_info_args(price, size).map_err(|e| anyhow::anyhow!("{e:#?}")))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let bid_cancels: HashSet<(u32, u64, u64)> = HashSet::from_iter(
        latest_state
            .bids
            .iter()
            .map(|bid| (bid.encoded_price, bid.base_remaining, bid.quote_remaining)),
    );

    let ask_cancels: HashSet<(u32, u64, u64)> = HashSet::from_iter(
        latest_state
            .asks
            .iter()
            .map(|ask| (ask.encoded_price, ask.base_remaining, ask.quote_remaining)),
    );

    let mut unique_bid_cancels = vec![];
    let mut unique_bid_posts = vec![];

    for args in post_bid_args {
        let o =
            to_order_info(args.0, args.1, args.2, args.3).map_err(|e| anyhow::anyhow!("{e:#?}"))?;

        if bid_cancels.contains(&(o.encoded_price.as_u32(), o.base_atoms, o.quote_atoms)) {
            // Do nothing
        } else {
            unique_bid_cancels.push(CancelOrderInstructionData::new())
            // keep the cancel and post
            unique_bid_cancels.push(/* create the cancel here and push it */);
            // push the post
            unique_bid_posts.push(/* create the post here and push */)
        }
    }

    // bid_posts.iter().filter(|new_bid| bid_cancels.)
    // ask_posts.iter().filter

    let ask_cancels: HashSet<(u32, u64, u64)> = HashSet::from_iter(
        latest_state
            .asks
            .iter()
            .map(|bid| (bid.encoded_price, bid.base_remaining, bid.quote_remaining)),
    );
    (vec![], vec![])
}

fn get_normalized_mid_price(
    candlestick_response: OandaCandlestickResponse,
    expected_pair: &CurrencyPair,
    market_ctx: &MarketContext,
) -> anyhow::Result<Decimal> {
    let response_pair = &candlestick_response.instrument;
    if expected_pair != response_pair {
        anyhow::bail!("Maker and and candlestick response pair don't match. {expected_pair} != {response_pair}");
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

    Ok(normalize_non_atoms_price(
        latest_price,
        market_ctx.base.mint_decimals,
        market_ctx.quote.mint_decimals,
    ))
}

/// Converts a token price not denominated in atoms to a token price denominated in atoms using
/// exponentiation based on the base and quote token's decimals.
fn normalize_non_atoms_price(
    non_atoms_price: Decimal,
    base_decimals: u8,
    quote_decimals: u8,
) -> Decimal {
    decimal_pow10_i16(
        non_atoms_price,
        quote_decimals as i16 - base_decimals as i16,
    )
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use crate::maker_context::normalize_non_atoms_price;

    #[test]
    fn varying_decimal_pair() {
        // Equal decimals => do nothing.
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 6, 6), dec!(1.27));

        // 10 ^ (quote - base) == 10 ^ 1 == multiply by 10
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 5, 6), dec!(12.7));

        // 10 ^ (quote - base) == 10 ^ -1 == divide by 10
        assert_eq!(normalize_non_atoms_price(dec!(1.27), 6, 5), dec!(0.127));

        // 10 ^ (quote - base) == 10 ^ (19 - 11) == multiply by 10 ^ 8
        assert_eq!(
            normalize_non_atoms_price(dec!(1.27), 11, 19),
            dec!(127_000_000)
        );

        // 10 ^ (quote - base) == 10 ^ (11 - 19) = divide by 10 ^ 8
        assert_eq!(
            normalize_non_atoms_price(dec!(1.27), 19, 11),
            dec!(0.0000000127)
        );
    }
}
