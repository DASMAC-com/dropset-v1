//! Internal debugging helpers for local development and testing.

/// Debug macro that wraps the solana program log crate with a debug feature flag.
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(feature = "debug")]
        solana_program_log::log!($($arg)*)
    };
}
