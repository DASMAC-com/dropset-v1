use instruction_macros::ProgramInstructionEvent;

#[repr(u8)]
#[derive(ProgramInstructionEvent)]
#[program_id(crate::ID)]
#[rustfmt::skip]
pub enum DropsetEvent {
    #[args(instruction_tag: u8, "The tag of the instruction that emitted the following events.")]
    #[args(market: [u8; 32], "The market's pubkey.")]
    #[args(sender: [u8; 32], "The sender's pubkey.")]
    #[args(nonce: u64, "The market nonce.")]
    #[args(emitted_count: u16, "The number of events in the following event buffer.")]
    Header,
    #[args(trader: [u8; 32], "The trader's pubkey.")]
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]    
    Deposit,
    #[args(trader: [u8; 32], "The trader's pubkey.")]
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]    
    Withdraw,
}
