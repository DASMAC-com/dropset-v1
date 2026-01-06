//! Definitions and re-exports of all program instruction event structs and their respective pack
//! and unpack implementations.
//!
//! Since instruction data events share the same pack/unpack implementations for instruction data
//! as full-fledged instructions but are not actual instructions that can be invoked with accounts,
//! [`DropsetEventTag`] derives [`ProgramInstructionEvent`] instead of
//! [`instruction_macros::ProgramInstruction`].
//!
//! Notably, the differences in generated code are:
//!
//! - the struct definitions and their `pack` implementations are feature-independent and thus not
//!   namespaced; e.g. [`HeaderEventInstructionData`], [`HeaderEventInstructionData::pack`]
//! - the `unpack` methods are only generated for the `client` feature, since instruction events
//!   should never be viewed/parsed on-chain.
//! - variants cannot define accounts
//! - invocation methods or anything that uses instruction accounts are not generated

#[cfg(test)]
mod tests;

use instruction_macros::ProgramInstructionEvent;

use crate::error::DropsetError;

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
    #[args(market: [u8; 32], "The market's pubkey.")]
    HeaderEvent,
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    #[args(seat_sector_index: u32, "The user's (possibly newly registered) market seat sector index.")]
    DepositEvent,
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    WithdrawEvent,
    #[args(market: [u8; 32], "The newly registered market.")]
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

impl TryFrom<u8> for DropsetEventTag {
    type Error = DropsetError;

    #[inline(always)]
    fn try_from(tag: u8) -> Result<Self, Self::Error> {
        DropsetEventTag_try_from_tag!(tag, DropsetError::InvalidInstructionEventTag)
    }
}
