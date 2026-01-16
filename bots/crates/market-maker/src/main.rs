//! Creates a market making bot that utilizes the market making strategy define in the
//! Avellaneda-Stoikov paper here: <https://people.orie.cornell.edu/sfs33/LimitOrderBook.pdf>

use std::sync::LazyLock;

const RISK_AVERSION: f64 = 1.0;
const VOLATILITY_ESTIMATE: f64 = 1.0;
const TIME_HORIZON: f64 = 300.0;
const FILL_DECAY: f64 = 1.5;

pub fn volatility_estimate_squared() -> &'static f64 {
    static VE: LazyLock<f64> = LazyLock::new(|| VOLATILITY_ESTIMATE.powf(2f64));
    LazyLock::force(&VE)
}

/// Calculates the reservation price, also known as the indifference price and the central price.
///
/// The reservation price is the price at which a maker is indifferent to buying or selling a single
/// unit of the base asset.
///
/// Put simply, it is a function of the pair's mid price and the maker's current base inventory (how
/// long or short they are).
///
/// Thus the function arguments are:
///
/// - `mid_price: f64`
/// - `base_inventory: i64`
///
/// Also depends on various tuning parameters. The A-S paper defines them as:
/// - the maker's risk aversion `γ`
/// - a volatility estimate for the market `σ`
/// - Time remaining, aka the effective time horizon `T - t`
///
/// Equation (3.17):
///
/// ```text
/// r = mid_price - (base_inventory · risk_aversion · volatility_estimate² · (T - t))
/// ```
fn calculate_reservation_price(mid_price: f64, base_inventory: i64) -> f64 {
    let base_inventory_f64 = base_inventory as f64;

    mid_price - (base_inventory_f64 * RISK_AVERSION * volatility_estimate_squared() * TIME_HORIZON)
}

/// Calculates half of the total spread.
///
/// Equation (3.18):
///
/// total_spread = (risk_aversion · volatility_estimate² · time_horizon)
///                + (2 / risk_aversion) · ln(1 + (risk_aversion / fill_decay))
///
/// Thus half that value is half the spread.
fn half_spread() -> f64 {
    static HALF_SPREAD: LazyLock<f64> = LazyLock::new(|| {
        let spread = (RISK_AVERSION * volatility_estimate_squared() * TIME_HORIZON)
            + (2.0 / RISK_AVERSION) * (1.0 + (RISK_AVERSION / FILL_DECAY)).ln();

        spread / 2.0
    });

    *LazyLock::force(&HALF_SPREAD)
}

struct MakerContext {
    /// The quote/base mark price, aka the mid price.
    mark_price: f64,
    /// The total size of bids filled in base atoms.
    bid_fills: u64,
    /// The total size of asks filled in base atoms.
    ask_fills: u64,
}

fn bid_and_ask_price(ctx: &MakerContext) -> (f64, f64) {
    let reservation_price = calculate_reservation_price(
        ctx.mark_price,
        (ctx.bid_fills as i128 - ctx.ask_fills as i128) as i64,
    );

    let bid_price = reservation_price - half_spread();
    let ask_price = reservation_price + half_spread();
    (bid_price, ask_price)
}

fn main() {
    println!("Hello, world!");
}
