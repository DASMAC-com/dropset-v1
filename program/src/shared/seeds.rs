//! Defines helper macros for deriving PDA addresses.

#[macro_export]
macro_rules! market_seeds {
    ($base:expr, $quote:expr) => {
        &[
            $base.as_ref(),
            $quote.as_ref(),
            ::dropset_interface::seeds::market::MARKET_SEED_STR,
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
/// let base_mint = pinocchio_pubkey::pubkey!("11111111111111111111111111111111111111111111");
/// let quote_mint = pinocchio_pubkey::pubkey!("22222222222222222222222222222222222222222222");
/// let signer: Signer = market_signer!(base_mint, quote_mint, bump);
/// ```
#[macro_export]
macro_rules! market_signer {
    ( $base_mint:expr, $quote_mint:expr, $bump:expr ) => {
        ::pinocchio::instruction::Signer::from(&::pinocchio::seeds!(
            $base_mint.as_ref(),
            $quote_mint.as_ref(),
            ::dropset_interface::seeds::market::MARKET_SEED_STR,
            &[$bump]
        ))
    };
}

/// # Example
///
/// ```
/// use dropset::event_authority_signer;
/// use pinocchio::instruction::Signer;
///
/// let signer: Signer = event_authority_signer!();
/// ```
#[macro_export]
macro_rules! event_authority_signer {
    ( ) => {
        ::pinocchio::instruction::Signer::from(&::pinocchio::seeds!(
            ::dropset_interface::seeds::event_authority::EVENT_AUTHORITY_SEED_STR,
            &[::dropset_interface::seeds::event_authority::BUMP]
        ))
    };
}
