use client::{
    context::{
        market::MarketContext,
        token::TokenContext,
    },
    transactions::CustomRpcClient,
};
use itertools::Itertools;
use rust_decimal::Decimal;
use solana_address::Address;
use solana_keypair::Signer;
use solana_sdk::{
    message::Instruction,
    signature::Keypair,
};
use transaction_parser::views::{
    try_market_view_all_from_owner_and_data,
    MarketViewAll,
};

use crate::{
    calculate_spreads::{
        half_spread,
        reservation_price,
    },
    maker_context::utils::{
        get_normalized_mid_price,
        log_orders,
    },
    oanda::{
        CurrencyPair,
        OandaCandlestickResponse,
    },
};

pub mod maker_state;
pub mod order_as_key;
pub mod order_flow;
pub mod utils;

pub use maker_state::*;
pub use order_as_key::*;
pub use order_flow::*;

const ORDER_SIZE: u64 = 1_000;

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
    pub async fn init(
        rpc: &CustomRpcClient,
        maker: Keypair,
        base_mint: Address,
        quote_mint: Address,
        pair: CurrencyPair,
        base_target_atoms: u64,
        initial_price_feed_response: OandaCandlestickResponse,
    ) -> anyhow::Result<Self> {
        let base_account = rpc.client.get_account(&base_mint).await?;
        let base =
            TokenContext::from_account_data(base_mint, base_account.owner, &base_account.data)?;

        let quote_account = rpc.client.get_account(&quote_mint).await?;
        let quote =
            TokenContext::from_account_data(quote_mint, quote_account.owner, &quote_account.data)?;

        let market_ctx = MarketContext::new(base, quote);

        let market_account = rpc.client.get_account(&market_ctx.market).await?;
        let market =
            try_market_view_all_from_owner_and_data(market_account.owner, &market_account.data)?;
        let latest_state = MakerState::new_from_market(maker.pubkey(), market)?;
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
            .collect_vec();

        Ok(ixns)
    }

    pub fn update_maker_state(&mut self, new_market_state: MarketViewAll) -> anyhow::Result<()> {
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
