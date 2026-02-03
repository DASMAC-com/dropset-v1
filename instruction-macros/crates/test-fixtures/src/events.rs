use instruction_macros::{
    Pack,
    ProgramInstructionEvent,
    Unpack,
};

#[repr(u8)]
#[derive(ProgramInstructionEvent)]
#[program_id(crate::ID)]
#[rustfmt::skip]
pub enum DropsetEvent {
    #[args(instruction_tag: u8, "The tag of the instruction that emitted the following events.")]
    #[args(market: Address, "The market's address.")]
    #[args(sender: Address, "The sender's address.")]
    #[args(nonce: u64, "The market nonce.")]
    #[args(emitted_count: u16, "The number of events in the following event buffer.")]
    Header,
    #[args(trader: Address, "The trader's address.")]
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    Deposit,
    #[args(trader: Address, "The trader's address.")]
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    Withdraw,
    #[args(big_info_1: BigOrderInfo, "Big order info 1.")]
    #[args(big_info_2: BigOrderInfo, "Big order info 2.")]
    #[args(big_info_3: BigOrderInfo, "Big order info 3.")]
    BigOrderInfos,
}

#[repr(C)]
#[derive(Debug, Clone, Pack, Unpack, PartialEq, Eq)]
pub struct BigOrderInfo {
    pub deposit_1: DepositInstructionData,
    pub bool_1: bool,
    pub deposit_2: DepositInstructionData,
    pub random_field: u64,
    pub withdraw_1: WithdrawInstructionData,
    pub withdraw_2: WithdrawInstructionData,
    pub bool_2: bool,
    pub field_2: u64,
}

#[cfg(test)]
mod tests {
    use instruction_macros::{
        Pack,
        Tagged,
        Unpack,
    };
    use solana_address::Address;

    use super::*;

    crate::create_big_order_info_pack_and_unpack_test!();
}
