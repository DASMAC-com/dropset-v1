//! The parameters in this file are arbitrary configuration knobs that should be relative to the
//! expected price of a market and its characteristics like volatility and order book depth.

use rust_decimal::{
    dec,
    Decimal,
};

/// Risk-aversion parameter (γ). Higher => stronger inventory penalty. This value skews quotes more
/// to mean-revert inventory.
pub const RISK_AVERSION: Decimal = dec!(0.1);

/// Volatility estimate (σ) in *price units per sqrt(second)* (i.e. stddev of mid-price change over
/// 1 second). If you want “X% per second”, set `sigma = mid_price * X` (e.g. 0.01% => X=1e-4).
pub const VOLATILITY_ESTIMATE: Decimal = dec!(0.0001);

/// Effective time horizon in seconds (T - t or τ). Longer => more inventory risk => wider spread +
/// stronger skew.
pub const TIME_HORIZON: Decimal = dec!(0.1);

/// Smallest representable increment of price utilized by the model (aka one tick), in price units.
/// This can match the smallest representable increment on-chain or be arbitrary- but it must be
/// consistent with [`VOLATILITY_ESTIMATE`].
pub const PRICE_STEP: Decimal = dec!(0.0001);

/// Human-friendly fill-decay knob:
/// This value represents how many [`PRICE_STEP`]s away from mid price until the fill intensity
/// drops by e⁻¹.
/// Converted into `k` (units: 1/price) for λ(δ)=A·exp(-k·δ).
pub const FILL_DECAY_STEPS: Decimal = dec!(10);
