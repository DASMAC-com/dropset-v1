//! Calculation functions used to implement the market making strategy defined in the
//! Avellaneda-Stoikov paper here: <https://people.orie.cornell.edu/sfs33/LimitOrderBook.pdf>
//!
//! Tune the model parameters to your specific market's characteristics in
//! [`crate::model_parameters`].

use std::sync::LazyLock;

use rust_decimal::{
    dec,
    prelude::ToPrimitive,
    Decimal,
};

use crate::model_parameters::*;

/// Calculates the reservation price, also known as the indifference price and the central price.
///
/// The reservation price is the price at which a maker is indifferent to buying or selling a single
/// unit of the base asset.
///
/// Put simply, it is a function of the pair's mid price and `q`, a value that represents how long
/// or short the maker is.
///
/// This calculation also depends on various tuning parameters. The A-S model defines them as:
/// - the maker's risk aversion `γ`
/// - a volatility estimate for the market `σ`
/// - Time remaining, aka the effective time horizon `T - t`
///
/// Equation (3.17):
///
/// ```text
/// r = mid_price - (q · risk_aversion · volatility_estimate² · (T - t))
/// ```
pub fn reservation_price(mid_price: Decimal, q: Decimal) -> Decimal {
    mid_price - (q * RISK_AVERSION * volatility_estimate_squared() * TIME_HORIZON)
}

fn ln_decimal_f64(d: Decimal) -> Option<Decimal> {
    if d <= Decimal::ZERO {
        return None;
    }

    d.to_f64().and_then(|v| Decimal::from_f64_retain(v.ln()))
}

/// Calculates half of the total spread.
///
/// Equation (3.18):
///
/// total_spread = (risk_aversion · volatility_estimate² · time_horizon)
///                + (2 / risk_aversion) · ln(1 + (risk_aversion / fill_decay))
///
/// Thus half that value is half the spread.
pub fn half_spread() -> Decimal {
    static HALF_SPREAD: LazyLock<Decimal> = LazyLock::new(|| {
        let spread = (RISK_AVERSION * volatility_estimate_squared() * TIME_HORIZON)
            + (dec!(2.0) / RISK_AVERSION)
                * ln_decimal_f64(dec!(1.0) + (RISK_AVERSION / fill_decay()))
                    .expect("Should calculate natural log");

        spread / dec!(2.0)
    });

    *LazyLock::force(&HALF_SPREAD)
}

fn volatility_estimate_squared() -> Decimal {
    static VOL_SQ: LazyLock<Decimal> = LazyLock::new(|| VOLATILITY_ESTIMATE * VOLATILITY_ESTIMATE);

    *LazyLock::force(&VOL_SQ)
}

/// The model `k` value representing the distance from mid price indicating where fill intensity
/// drops off.
fn fill_decay() -> Decimal {
    static K: LazyLock<Decimal> = LazyLock::new(|| {
        // k = 1 / (steps * price_step)
        Decimal::ONE / (FILL_DECAY_STEPS * PRICE_STEP)
    });

    *LazyLock::force(&K)
}
