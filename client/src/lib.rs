//! Client-side utilities for interacting with Dropset programs.
//!
//! Includes context helpers, pretty-printing utilities, and PDA derivations.

pub mod context;
pub mod logs;
pub mod pda;
pub mod pretty;
pub mod test_accounts;
pub mod transactions;
pub mod views;

pub use logs::LogColor;
