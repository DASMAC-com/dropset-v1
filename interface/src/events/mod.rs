//! Definitions and re-exports of all program instruction event structs and their respective pack
//! and unpack implementations.
//!
//! Note that [`DropsetEventTag`] derives [`ProgramInstructionEvent`] instead of
//! [`instruction_macros::ProgramInstruction`].

#[cfg(test)]
mod tests;

use instruction_macros::ProgramInstructionEvent;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, ProgramInstructionEvent)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
#[cfg_attr(feature = "client", derive(strum_macros::Display))]
#[program_id(crate::program::ID)]
#[rustfmt::skip]
pub enum DropsetEventTag {
    #[args(instruction_tag: u8, "The tag of the instruction that emitted the following events.")]
    #[args(emitted_count: u16, "The number of events in the following event buffer.")]
    #[args(num_events: u64, "The market's final, total number of events.")]
    #[args(market: Address, "The market's address.")]
    HeaderEvent,
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    #[args(seat_sector_index: u32, "The user's (possibly newly registered) market seat sector index.")]
    DepositEvent,
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    WithdrawEvent,
    #[args(market: Address, "The newly registered market.")]
    RegisterMarketEvent,
    #[args(is_bid: bool, "Whether or not the order is a bid. If false, the order is an ask.")]
    #[args(user_seat_sector_index: u32, "The user's market seat sector index.")]
    #[args(order_sector_index: u32, "The posted order's sector index.")]
    #[args(base_atoms: u64, "The size of the order's base atoms to fill.")]
    #[args(quote_atoms: u64, "The size of the order's quote atoms to fill.")]
    PostOrderEvent,
    #[args(is_bid: bool, "Whether or not the order is a bid. If false, the order is an ask.")]
    #[args(user_seat_sector_index: u32, "The user's market seat sector index.")]
    CancelOrderEvent,
    #[args(order_size: u64, "The order size in atoms.")]
    #[args(is_buy: bool, "Whether or not the order is a market buy. If not, it's a market sell.")]
    #[args(is_base: bool, "Whether or not the order size is denominated in base. If not, it's in quote.")]
    #[args(base_filled: u64, "The amount of base atoms filled.")]
    #[args(quote_filled: u64, "The amount of quote atoms filled.")]
    MarketOrderEvent,
    #[args(user_seat_sector_index: u32, "The user's market seat sector index.")]
    CloseSeatEvent,
}
