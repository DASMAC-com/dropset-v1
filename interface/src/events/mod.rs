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
    #[args(market: [u8; 32], "The market's pubkey.")]
    #[args(nonce: u64, "The market nonce.")]
    #[args(emitted_count: u16, "The number of events in the following event buffer.")]
    Header,
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]    
    #[args(seat_sector_index: u32, "The user's (possibly newly registered) market seat sector index.")]
    Deposit,
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]    
    Withdraw,
    #[args(market: [u8; 32], "The newly registered market.")]
    RegisterMarket,
    #[args(seat_sector_index: u32, "The user's market seat sector index.")]
    CloseSeat,
}

impl TryFrom<u8> for DropsetEventTag {
    type Error = DropsetError;

    #[inline(always)]
    fn try_from(tag: u8) -> Result<Self, Self::Error> {
        DropsetEventTag_try_from_tag!(tag, DropsetError::InvalidInstructionEventTag)
    }
}
