use std::{
    collections::HashMap,
    hash::Hash,
};

use client::{
    context::market::MarketContext,
    print_kv,
    transactions::CustomRpcClient,
};
use dropset_interface::{
    instructions::{
        CancelOrderInstructionData,
        PostOrderInstructionData,
    },
    state::{
        asks_dll::AskOrders,
        bids_dll::BidOrders,
        order::BookSide,
        sector::SectorIndex,
        user_order_sectors::OrderSectors,
    },
};
use itertools::Itertools;
use price::{
    client_helpers::{
        decimal_pow10_i16,
        to_order_info_args,
        try_encoded_u32_to_decoded_decimal,
    },
    to_order_info,
    OrderInfo,
    OrderInfoArgs,
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

/// Tracks the market maker's seat, bids, asks, and total base and quote inventory for a market.
///
/// To simplify this struct's interface, the market maker must have already been registered prior
/// to instantiating this struct.
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

/// Find an order in a collection of orders given the price, the [`BookSide`], and the seat index.
fn find_order<T: BookSide>(
    price: u32,
    orders: &[OrderView],
    seat_index: SectorIndex,
) -> Option<OrderView> {
    let i = orders
        .binary_search_by(|o| T::cmp_prices(o.encoded_price, price))
        .ok()?;

    // `core::slice`'s binary search can return any of multiple matches.
    // Search forwards and backwards from the found index, stopping when the price changes.
    // Return when the order seat matches the passed seat index.
    //
    // This can be achieved by chaining and then searching the three iterators for the seat index:
    // 1. The first, found element: [i]
    // 2. The elements before: [i - 1, i - 2, ..] while the same `price`
    // 3. The elements after: [i + 1, i + 2, ..] while the same `price`

    let first_found = &orders[i];

    let before = orders[..i]
        .iter()
        .rev()
        .take_while(|o| o.encoded_price == price);

    let after = orders[i + 1..]
        .iter()
        .take_while(|o| o.encoded_price == price);

    std::iter::once(first_found)
        .chain(before)
        .chain(after)
        .find(|o| o.user_seat == seat_index)
        .cloned()
}

impl MakerState {
    /// Creates the market maker's state based on the passed [`MarketViewAll`] state.
    /// If the maker doesn't have a seat registered yet this will fail.
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

        // Map each bid price to its corresponding order.
        let bids = bid_prices
            .iter()
            .map(|price| find_order::<BidOrders>(*price, &market.bids, seat.index))
            .collect::<Option<Vec<_>>>()
            .expect("Should find the bid");

        // Map each ask price to its corresponding order.
        let asks = ask_prices
            .iter()
            .map(|price| find_order::<AskOrders>(*price, &market.asks, seat.index))
            .collect::<Option<Vec<_>>>()
            .expect("Should find the ask");

        // Sum the maker's base inventory by adding the seat balance + the bid collateral amounts.
        let base_inventory = bids
            .iter()
            .fold(seat.base_available, |v, order| v + order.base_remaining);

        // Sum the maker's quote inventory by adding the seat balance + the ask collateral amounts.
        let quote_inventory = asks
            .iter()
            .fold(seat.quote_available, |v, order| v + order.quote_remaining);

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
    maker_address: Address,
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
        let (bid_price, ask_price) = self.get_bid_and_ask_prices();

        let (cancels, posts) = get_non_redundant_order_flow(
            self.latest_state.bids.clone(),
            self.latest_state.asks.clone(),
            vec![(bid_price, ORDER_SIZE)],
            vec![(ask_price, ORDER_SIZE)],
            self.latest_state.seat.index,
        )?;

        log_orders(&posts, &cancels)?;

        let ixns = cancels
            .into_iter()
            .map(|cancel| self.market_ctx.cancel_order(self.maker_address, cancel))
            .chain(
                posts
                    .into_iter()
                    .map(|post| self.market_ctx.post_order(self.maker_address, post)),
            )
            .map(Instruction::from)
            .collect_vec();

        Ok(ixns)
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

#[derive(Hash, Eq, PartialEq)]
struct OrderAsKey {
    encoded_price: u32,
    base: u64,
    quote: u64,
}

impl From<OrderInfo> for OrderAsKey {
    fn from(o: OrderInfo) -> Self {
        Self {
            encoded_price: o.encoded_price.as_u32(),
            base: o.base_atoms,
            quote: o.quote_atoms,
        }
    }
}

impl From<OrderView> for OrderAsKey {
    fn from(o: OrderView) -> Self {
        Self {
            encoded_price: o.encoded_price,
            base: o.base_remaining,
            quote: o.quote_remaining,
        }
    }
}

/// Returns a pair of vectors that represents the uniquely keyed values in each hashmap.
///
/// Example in pseudo-code, where each pair represents a (k, v) pair in a hashmap.
///
/// let res = split_symmetric_difference(
///     ((1, a), (2, b), (3, c)),
///     ((3, c), (4, d), (5, e))
/// );
///
/// res == ([a, b], [d, e])
fn split_symmetric_difference<'a, K: Eq + Hash, V1, V2>(
    a: &'a HashMap<K, V1>,
    b: &'a HashMap<K, V2>,
) -> (Vec<&'a V1>, Vec<&'a V2>) {
    (
        a.iter()
            .filter(|(k, _)| !b.contains_key(k))
            .map(|(_, v)| v)
            .collect(),
        b.iter()
            .filter(|(k, _)| !a.contains_key(k))
            .map(|(_, v)| v)
            .collect(),
    )
}

fn to_order_args_map(
    prices_and_sizes: Vec<(Decimal, u64)>,
) -> anyhow::Result<HashMap<OrderAsKey, OrderInfoArgs>> {
    prices_and_sizes
        .into_iter()
        .map(|(price, size)| {
            let args = to_order_info_args(price, size)?;
            let order_info = to_order_info(args.clone())?;
            Ok((order_info.into(), args))
        })
        .collect()
}

fn to_order_view_map(orders: Vec<OrderView>) -> HashMap<OrderAsKey, OrderView> {
    orders
        .into_iter()
        .map(|order| (order.clone().into(), order))
        .collect()
}

/// Given the collections of bids/asks to cancel and bids/asks to post, determine which orders would
/// be redundant and then filter them out from the set of resulting instructions.
///
/// That is, if an order would be canceled and then reposted, the cancel and post instruction are
/// both redundant and should be filtered out.
///
/// The bids and asks in the latest stored state might be stale due to fills.
/// This will cause the cancel order attempts to fail and should be expected intermittently.
fn get_non_redundant_order_flow(
    bids_to_cancel: Vec<OrderView>,
    asks_to_cancel: Vec<OrderView>,
    // Vec of (price, size) tuples.
    bids_to_post: Vec<(Decimal, u64)>,
    // Vec of (price, size) tuples.
    asks_to_post: Vec<(Decimal, u64)>,
    maker_seat_index: SectorIndex,
) -> anyhow::Result<(
    Vec<CancelOrderInstructionData>,
    Vec<PostOrderInstructionData>,
)> {
    // Map the existing maker's key-able order infos to their respective orders.
    // These will be the orders that are canceled.
    let bid_cancels = to_order_view_map(bids_to_cancel);
    let ask_cancels = to_order_view_map(asks_to_cancel);

    // Map the incoming (to-be-posted) key-able order infos to their respective order info args.
    let bid_posts = to_order_args_map(bids_to_post)?;
    let ask_posts = to_order_args_map(asks_to_post)?;

    // Retain only the unique values in two hash maps `a` and `b`, where each item in `a` does not
    // have a corresponding matching key in `b`.
    let (c_ask, p_ask, c_bid, p_bid) = (&ask_cancels, &ask_posts, &bid_cancels, &bid_posts);
    let (unique_bid_posts, unique_bid_cancels) = split_symmetric_difference(p_bid, c_bid);
    let (unique_ask_posts, unique_ask_cancels) = split_symmetric_difference(p_ask, c_ask);

    let cancels = unique_bid_cancels
        .iter()
        .map(|c| CancelOrderInstructionData::new(c.encoded_price, true, maker_seat_index))
        .chain(
            unique_ask_cancels
                .iter()
                .map(|c| CancelOrderInstructionData::new(c.encoded_price, false, maker_seat_index)),
        )
        .collect_vec();

    let posts = unique_bid_posts
        .iter()
        .map(|p| {
            PostOrderInstructionData::new(
                p.price_mantissa,
                p.base_scalar,
                p.base_exponent_biased,
                p.quote_exponent_biased,
                true,
                maker_seat_index,
            )
        })
        .chain(unique_ask_posts.iter().map(|p| {
            PostOrderInstructionData::new(
                p.price_mantissa,
                p.base_scalar,
                p.base_exponent_biased,
                p.quote_exponent_biased,
                false,
                maker_seat_index,
            )
        }))
        .collect_vec();

    Ok((cancels, posts))
}

fn get_normalized_mid_price(
    candlestick_response: OandaCandlestickResponse,
    expected_pair: &CurrencyPair,
    market_ctx: &MarketContext,
) -> anyhow::Result<Decimal> {
    let response_pair = &candlestick_response.instrument;
    if expected_pair != response_pair {
        anyhow::bail!(
            "Maker and candlestick response pair don't match. {expected_pair} != {response_pair}"
        );
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
                .ok_or_else(|| anyhow::anyhow!("`mid` price not found in the last candlestick."))?
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

fn log_orders(
    posts: &[PostOrderInstructionData],
    cancels: &[CancelOrderInstructionData],
) -> anyhow::Result<()> {
    for cancel in cancels.iter() {
        let side = if cancel.is_bid { "bid" } else { "ask" };
        let decimal_price = try_encoded_u32_to_decoded_decimal(cancel.encoded_price)?;
        print_kv!(format!("Canceling {side} at"), format!("{decimal_price}"),);
    }

    for post in posts.iter() {
        let side = if post.is_bid { "bid" } else { "ask" };
        let encoded_price = to_order_info(
            (
                post.price_mantissa,
                post.base_scalar,
                post.base_exponent_biased,
                post.quote_exponent_biased,
            )
                .into(),
        )?
        .encoded_price;
        let decimal_price = try_encoded_u32_to_decoded_decimal(encoded_price.as_u32())?;
        print_kv!(format!("Posting {side} at"), format!("{decimal_price}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use dropset_interface::state::{
        asks_dll::AskOrders,
        bids_dll::BidOrders,
    };
    use itertools::Itertools;
    use rust_decimal::dec;
    use transaction_parser::views::OrderView;

    use crate::maker_context::{
        find_order,
        normalize_non_atoms_price,
    };

    fn create_order(price: u32, seat: u32) -> OrderView {
        OrderView {
            prev_index: 0,
            index: 0,
            next_index: 0,
            encoded_price: price,
            user_seat: seat,
            base_remaining: 0,
            quote_remaining: 0,
        }
    }

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

    #[test]
    fn find_asks_with_duplicate_prices() {
        // Note in the current order book implementation, only the price is always sorted. The seat
        // indices could be random because it's based on time priority, not seat index.
        // Asks are in ascending order because lower prices have a higher priority.
        let prices: [u32; 8] = [0, 0, 0, 1, 1, 1, 2, 3];
        let seats: [u32; 8] = [8, 9, 3, 1, 4, 7, 2, 6];

        let pairs = prices.into_iter().zip(seats).collect_vec();

        let asks = pairs
            .iter()
            .copied()
            .map(|(price, seat)| create_order(price, seat))
            .collect_vec();

        for (price, seat) in pairs {
            let order = find_order::<AskOrders>(price, &asks, seat).expect("Should find seat");
            assert_eq!(order.encoded_price, price);
            assert_eq!(order.user_seat, seat);
            assert!(find_order::<AskOrders>(price, &asks, 10000).is_none());
            assert!(find_order::<AskOrders>(10000, &asks, seat).is_none());
        }
    }

    #[test]
    fn find_bids_with_duplicate_prices() {
        // Note in the current order book implementation, only the price is always sorted. The seat
        // indices could be random because it's based on time priority, not seat index.
        // Bids are in descending order because higher prices have a higher priority.
        let prices: [u32; 8] = [3, 2, 1, 1, 1, 0, 0, 0];
        let seats: [u32; 8] = [8, 9, 3, 1, 4, 7, 2, 6];

        let pairs = prices.into_iter().zip(seats).collect_vec();

        let bids = pairs
            .iter()
            .copied()
            .map(|(price, seat)| create_order(price, seat))
            .collect_vec();

        for (price, seat) in pairs {
            let order = find_order::<BidOrders>(price, &bids, seat).expect("Should find seat");
            assert_eq!(order.encoded_price, price);
            assert_eq!(order.user_seat, seat);
            assert!(find_order::<BidOrders>(price, &bids, 10000).is_none());
            assert!(find_order::<BidOrders>(10000, &bids, seat).is_none());
        }
    }
}
