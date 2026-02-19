use client::{
    context::market::{
        BookSide,
        Denomination,
    },
    e2e_helpers::{
        test_accounts,
        E2e,
        Trader,
    },
    single_signer_instruction::SingleSignerInstruction,
    transactions::ParsedTransactionWithEvents,
};
use dropset_interface::{
    events::MarketOrderEventInstructionData,
    instructions::{
        MarketOrderInstructionData,
        PostOrderInstructionData,
    },
    state::sector::NIL,
};
use itertools::Itertools;
use price::{
    biased_exponent,
    to_order_info,
    OrderInfo,
    OrderInfoArgs,
};
use solana_address::Address;
use solana_sdk::{
    signature::Keypair,
    signer::Signer,
};
use transaction_parser::{
    events::dropset_event::DropsetEvent,
    views::{
        MarketSeatView,
        OrderView,
    },
};

pub struct Balances {
    base: u64,
    quote: u64,
}

#[derive(PartialEq, Eq, Debug)]
struct BalanceDelta {
    base: i128,
    quote: i128,
}

impl BalanceDelta {
    fn new(base: i128, quote: i128) -> Self {
        Self { base, quote }
    }

    fn from_orders(order_before: &OrderView, order_after: &OrderView) -> Self {
        Self {
            base: i128::from(order_after.base_remaining) - i128::from(order_before.base_remaining),
            quote: i128::from(order_after.quote_remaining)
                - i128::from(order_before.quote_remaining),
        }
    }

    fn from_balances(balances_before: &Balances, balances_after: &Balances) -> Self {
        Self {
            base: i128::from(balances_after.base) - i128::from(balances_before.base),
            quote: i128::from(balances_after.quote) - i128::from(balances_before.quote),
        }
    }

    fn from_seats(seat_before: &MarketSeatView, seat_after: &MarketSeatView) -> Self {
        Self {
            base: i128::from(seat_after.base_available) - i128::from(seat_before.base_available),
            quote: i128::from(seat_after.quote_available) - i128::from(seat_before.quote_available),
        }
    }
}

impl Balances {
    pub async fn fetch(e2e: &E2e, user: &Address) -> anyhow::Result<Self> {
        Ok(Self {
            base: e2e.get_base_balance(user).await?,
            quote: e2e.get_quote_balance(user).await?,
        })
    }

    pub fn check(&self, expected_base: u64, expected_quote: u64) -> anyhow::Result<()> {
        if self.base != expected_base {
            anyhow::bail!("Expected {} base, got {}.", expected_base, self.base);
        }
        if self.quote != expected_quote {
            anyhow::bail!("Expected {} quote, got {}.", expected_quote, self.quote);
        }

        Ok(())
    }
}

struct MarketSnapshot {
    /// The maker's order on the book. `None` if the order has been filled or doesn't exist.
    maker_order: Option<OrderView>,
    /// The taker's token balances.
    taker_balances: Balances,
    /// The maker's seat.
    maker_seat: MarketSeatView,
    /// The market's token balances.
    market_balances: Balances,
}

impl MarketSnapshot {
    pub async fn new(e2e: &E2e, order_ctx: &OrderContext<'_>) -> anyhow::Result<Self> {
        let order_info = order_ctx.order_info()?;
        let OrderContext {
            maker,
            maker_side,
            taker,
            ..
        } = order_ctx;
        let market = e2e.view_market().await?;

        let maker_seat = market
            .seats
            .iter()
            .find(|seat| seat.user == maker.pubkey())
            .ok_or(anyhow::anyhow!("Couldn't find maker seat"))?
            .clone();

        let maker_order = match maker_side {
            BookSide::Ask => market.asks.iter(),
            BookSide::Bid => market.bids.iter(),
        }
        .find(|order| {
            order.encoded_price == order_info.encoded_price && order.user_seat == maker_seat.index
        })
        .cloned();

        let taker_balances = Balances::fetch(e2e, &taker.pubkey()).await?;
        let market_balances = Balances::fetch(e2e, &e2e.market.market).await?;

        Ok(Self {
            maker_order,
            taker_balances,
            maker_seat,
            market_balances,
        })
    }
}

struct OrderContext<'a> {
    pub maker: &'a Keypair,
    pub taker: &'a Keypair,
    pub order_info_args: OrderInfoArgs,
    pub maker_side: BookSide,
    // The number of taker fills required to fully fill the maker order.
    pub num_taker_fills: u64,
}

impl OrderContext<'_> {
    pub fn order_info(&'_ self) -> anyhow::Result<OrderInfo> {
        to_order_info(self.order_info_args.clone())
            .or(Err(anyhow::anyhow!("Couldn't create order info")))
    }

    pub fn taker_size_base(&self) -> anyhow::Result<u64> {
        Ok(self.order_info()?.base_atoms / self.num_taker_fills)
    }

    pub fn taker_size_quote(&self) -> anyhow::Result<u64> {
        Ok(self.order_info()?.quote_atoms / self.num_taker_fills)
    }
}

async fn run_partial_fill(
    e2e: &E2e,
    ctx: &OrderContext<'_>,
    denomination: Denomination,
) -> anyhow::Result<ParsedTransactionWithEvents> {
    let taker_is_market_buy = matches!(ctx.maker_side, BookSide::Ask);

    e2e.market
        .market_order(
            ctx.taker.pubkey(),
            MarketOrderInstructionData::new(
                match denomination {
                    Denomination::Base => ctx.taker_size_base()?,
                    Denomination::Quote => ctx.taker_size_quote()?,
                },
                taker_is_market_buy,
                denomination.is_base(),
            ),
        )
        .send_single_signer(&e2e.rpc, ctx.taker)
        .await
}

/// Registers a market maker seat and then posts an order based on the passed order context.
async fn post_maker_order(
    e2e: &E2e,
    ctx: &OrderContext<'_>,
) -> anyhow::Result<ParsedTransactionWithEvents> {
    let order_info = ctx.order_info()?;
    let market = &e2e.market;
    let maker = ctx.maker;

    let deposit_instruction = match ctx.maker_side {
        BookSide::Ask => market.deposit_base(maker.pubkey(), order_info.base_atoms, NIL),
        BookSide::Bid => market.deposit_quote(maker.pubkey(), order_info.quote_atoms, NIL),
    };

    deposit_instruction
        .send_single_signer(&e2e.rpc, ctx.maker)
        .await?;

    // The maker should have the first and only seat in the market.
    let market = e2e.view_market().await?;
    let maker_seat = market.seats.first().expect("Should have one market seat");
    assert_eq!(maker_seat.user, maker.pubkey());

    e2e.market
        .post_order(
            ctx.maker.pubkey(),
            PostOrderInstructionData::new(
                ctx.order_info_args.clone(),
                matches!(ctx.maker_side, BookSide::Bid),
                maker_seat.index,
            ),
        )
        .send_single_signer(&e2e.rpc, ctx.maker)
        .await
}

/// Initializes the `E2e` helper and returns the maker and taker's balances, respectively.
async fn initialize_traders_and_market(ctx: &OrderContext<'_>) -> anyhow::Result<E2e> {
    let order_info =
        to_order_info(ctx.order_info_args.clone()).map_err(|e| anyhow::anyhow!("{e:?}"))?;

    let base_atoms = order_info.base_atoms;
    let quote_atoms = order_info.quote_atoms;

    let (maker_base, maker_quote) = match ctx.maker_side {
        BookSide::Ask => (base_atoms, 0),
        BookSide::Bid => (0, quote_atoms),
    };

    let (taker_base, taker_quote) = match ctx.maker_side {
        BookSide::Ask => (0, quote_atoms),
        BookSide::Bid => (base_atoms, 0),
    };

    let e2e = E2e::new_traders_and_market(
        None,
        [
            Trader::new(ctx.maker, maker_base, maker_quote),
            Trader::new(ctx.taker, taker_base, taker_quote),
        ],
    )
    .await?;

    let maker_init = Balances::fetch(&e2e, &ctx.maker.pubkey()).await?;
    let taker_init = Balances::fetch(&e2e, &ctx.taker.pubkey()).await?;

    maker_init.check(maker_base, maker_quote)?;
    taker_init.check(taker_base, taker_quote)?;

    Ok(e2e)
}

fn assert_fill_deltas(
    side: BookSide,
    before: &MarketSnapshot,
    after: &MarketSnapshot,
    expected_base_filled: u64,
    expected_quote_filled: u64,
) {
    let base_filled = expected_base_filled as i128;
    let quote_filled = expected_quote_filled as i128;

    // Check the taker balance deltas.
    // If a taker market buys (fills an ask), they should receive base and spend quote.
    // If a taker market sells (fills a bid), they should spend base and receive quote.
    assert_eq!(
        BalanceDelta::from_balances(&before.taker_balances, &after.taker_balances),
        match side {
            BookSide::Ask => BalanceDelta::new(base_filled, -quote_filled),
            BookSide::Bid => BalanceDelta::new(-base_filled, quote_filled),
        }
    );

    // Ensure the market received the tokens.
    // The market balance is simply the inversion of the taker balance deltas, since the user
    // is sending the denominated asset to it and receiving the counter asset from it.
    assert_eq!(
        BalanceDelta::from_balances(&before.market_balances, &after.market_balances),
        match side {
            BookSide::Ask => BalanceDelta::new(-base_filled, quote_filled),
            BookSide::Bid => BalanceDelta::new(base_filled, -quote_filled),
        }
    );

    // Check the seat balance deltas.
    // If an ask fills, only the seat's quote increases. The base goes to the market.
    // If a bid fills, only the seat's base. The quote goes to the market.
    assert_eq!(
        BalanceDelta::from_seats(&before.maker_seat, &after.maker_seat),
        match side {
            BookSide::Ask => BalanceDelta::new(0, quote_filled),
            BookSide::Bid => BalanceDelta::new(base_filled, 0),
        }
    );

    // The order before should always exist, but the order after a fill may be removed from the book
    // so it's necessary to account for that.
    let before_order = before
        .maker_order
        .as_ref()
        .expect("The snapshotted order before the fill should exist");
    let order_delta = match &after.maker_order {
        // If the order exists, create the delta normally.
        Some(after_order) => BalanceDelta::from_orders(before_order, after_order),
        // If the order no longer exists, the delta is simply the amount remaining from before.
        None => BalanceDelta::new(
            0 - before_order.base_remaining as i128,
            0 - before_order.quote_remaining as i128,
        ),
    };

    // The order size simply decreases for any type of order and asset denomination.
    assert_eq!(order_delta, BalanceDelta::new(-base_filled, -quote_filled),);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let maker = test_accounts::acc_1111();
    let taker = test_accounts::acc_2222();

    const MAKER_SIZE_BASE: u64 = 500_000_000;
    const MAKER_SIZE_QUOTE: u64 = 55_000_000;

    for side in [BookSide::Ask, BookSide::Bid] {
        let ctx = OrderContext {
            maker,
            taker,
            order_info_args: OrderInfoArgs {
                price_mantissa: 11_000_000,
                base_scalar: 5,
                base_exponent_biased: biased_exponent!(8),
                quote_exponent_biased: biased_exponent!(0),
            },
            maker_side: side,
            num_taker_fills: 2,
        };

        let order_info = ctx.order_info()?;
        assert_eq!(order_info.base_atoms, MAKER_SIZE_BASE);
        assert_eq!(order_info.quote_atoms, MAKER_SIZE_QUOTE);

        // -----------------------------------------------------------------------------------------
        // Initialize the traders, register the market, and post the maker order.
        let e2e = initialize_traders_and_market(&ctx).await?;

        // Local helpers.
        let create_snapshot = async || MarketSnapshot::new(&e2e, &ctx).await;
        let taker_base_size = ctx.taker_size_base()?;
        let taker_quote_size = ctx.taker_size_quote()?;

        // Ensure no implicit truncation in fill sizes by checking that the posted order size is
        // divisible by the number of taker fills, since that's how taker size is calculated.
        assert_eq!(order_info.base_atoms % ctx.num_taker_fills, 0);
        assert_eq!(order_info.quote_atoms % ctx.num_taker_fills, 0);

        // -----------------------------------------------------------------------------------------
        // Post the maker order.
        post_maker_order(&e2e, &ctx).await?;

        // -----------------------------------------------------------------------------------------
        // Send the first taker order.
        let before_1 = create_snapshot().await?;
        let fill_1 = run_partial_fill(&e2e, &ctx, Denomination::Base).await?;
        let after_1 = create_snapshot().await?;

        assert_fill_deltas(side, &before_1, &after_1, taker_base_size, taker_quote_size);

        // -----------------------------------------------------------------------------------------
        // Send the second taker order.
        let before_2 = after_1;
        let fill_2 = run_partial_fill(&e2e, &ctx, Denomination::Quote).await?;
        let after_2 = create_snapshot().await?;

        assert_fill_deltas(side, &before_2, &after_2, taker_base_size, taker_quote_size);

        // Ensure that there's a single market order event in each transaction, both with the same
        // exact fill sizes.
        let get_market_order_event =
            |txn: &ParsedTransactionWithEvents| -> MarketOrderEventInstructionData {
                let mut orders: Vec<&MarketOrderEventInstructionData> = txn
                    .events
                    .iter()
                    .filter_map(|ev| match ev {
                        DropsetEvent::MarketOrder(m) => Some(m),
                        _ => None,
                    })
                    .collect_vec();
                assert_eq!(orders.len(), 1);
                orders.pop().unwrap().clone()
            };

        let event_1 = get_market_order_event(&fill_1);
        let event_2 = get_market_order_event(&fill_2);

        // Check the inputs against the emitted config.
        assert_eq!(event_1.order_size, taker_base_size);
        assert_eq!(event_2.order_size, taker_quote_size);
        assert!(event_1.is_base);
        assert!(!event_2.is_base);

        // Check that the direction matches the side.
        assert_eq!(event_1.is_buy, matches!(side, BookSide::Ask));
        assert_eq!(event_2.is_buy, matches!(side, BookSide::Ask));

        // Check that all the fill sizes are the same.
        assert_eq!(event_1.base_filled, event_2.base_filled);
        assert_eq!(event_1.quote_filled, event_2.quote_filled);
        assert_eq!(event_1.base_filled, taker_base_size);
        assert_eq!(event_1.quote_filled, taker_quote_size);
    }

    Ok(())
}
