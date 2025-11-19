//! Account context definitions for each `dropset` instruction.
//!
//! Each context groups and validates the accounts required by its corresponding instruction before
//! execution.

pub mod close_seat_context;
pub mod deposit_withdraw_context;
pub mod flush_events_context;
pub mod register_market_context;

/// The account infos necessary to emit events with the event buffer.
pub struct EventBufferContext<'a> {
    pub event_authority: &'a pinocchio::account_info::AccountInfo,
    pub market_account: crate::validation::market_account_info::MarketAccountInfo<'a>,
}
