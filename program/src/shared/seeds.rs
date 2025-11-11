//! Defines PDA seed constants and helper macros for deriving PDA addresses.

pub mod market {
    pub const MARKET_SEED_STR: &[u8] = b"market";
}

#[macro_export]
macro_rules! market_seeds {
    ($base:expr, $quote:expr) => {
        &[
            $base.as_ref(),
            $quote.as_ref(),
            $crate::shared::seeds::market::MARKET_SEED_STR,
        ]
    };
}

/// # Example
///
/// ```
/// use dropset::market_signer;
/// use pinocchio::instruction::Signer;
///
/// let bump: u8 = 0x10;
/// let signer: Signer = crate::market_signer!(base_mint, quote_mint, bump);
/// ```
#[macro_export]
macro_rules! market_signer {
    ( $base_mint:expr, $quote_mint:expr, $bump:expr ) => {
        pinocchio::instruction::Signer::from(&pinocchio::seeds!(
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            $crate::shared::seeds::market::MARKET_SEED_STR,
            &[$bump]
        ))
    };
}
