use instruction_macros::ProgramInstruction;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, ProgramInstruction)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
#[cfg_attr(feature = "client", derive(strum_macros::Display))]
#[program_id(crate::program::ID)]
#[rustfmt::skip]
pub enum DropsetEvent {
    #[args(instruction_tag: u8, "The tag of the instruction that emitted the following events.")]
    #[args(market: [u8; 32], "The market's pubkey.")]
    #[args(sender: [u8; 32], "The sender's pubkey.")]
    #[args(nonce: u64, "The market nonce.")]
    #[args(emitted_count: u16, "The number of events in the following event buffer.")]
    #[account(0, signer, name = "event_authority", desc = "The event authority account.")]
    Header,
    #[args(trader: [u8; 32], "The trader's pubkey.")]
    #[args(amount: u64, "The amount deposited.")]
    #[args(transfer_type: u8, "The token type: base or quote.")]    
    #[account(0, signer, name = "event_authority", desc = "The event authority account.")]
    Deposit,
    #[args(trader: [u8; 32], "The trader's pubkey.")]
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(transfer_type: u8, "The token type: base or quote.")]    
    #[account(0, signer, name = "event_authority", desc = "The event authority account.")]
    Withdraw,
}
