//! Instruction handlers for the `dropset` program.
//!
//! Routes decoded instructions to their corresponding handlers and encapsulates all
//! on-chain logic for each supported operation.

pub mod batch_replace;
pub mod cancel_order;
pub mod close_seat;
pub mod deposit;
pub mod flush_events;
pub mod market_order;
pub mod post_order;
pub mod register_market;
pub mod withdraw;

pub use batch_replace::process_batch_replace;
pub use cancel_order::process_cancel_order;
pub use close_seat::process_close_seat;
pub use deposit::process_deposit;
pub use flush_events::process_flush_events;
pub use market_order::process_market_order;
pub use post_order::process_post_order;
pub use register_market::process_register_market;
pub use withdraw::process_withdraw;
