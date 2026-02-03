use instruction_macros::{
    Pack,
    ProgramInstruction,
    Unpack,
};
use solana_address::Address;

use crate::create_big_order_info_test;

pub mod program_inner {
    use solana_address::Address;

    pub const ID: Address = Address::from_str_const("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");
}

const PROGRAM_ID: Address = program_inner::ID;

#[repr(u8)]
#[derive(ProgramInstruction)]
// Also works:
// #[program_id(PROGRAM_ID)]
// #[program_id(crate::ID)]
// #[program_id(crate::program::program_inner::ID)]
#[program_id(crate::program::PROGRAM_ID)]
#[rustfmt::skip]
pub enum ProgramDropsetInstruction {
    #[account(0, signer,   name = "user",                desc = "The user closing their seat.")]
    #[account(1, writable, name = "market_account",      desc = "The market account PDA.")]
    #[account(2, writable, name = "base_user_ata",       desc = "The user's associated base mint token account.")]
    #[account(3, writable, name = "quote_user_ata",      desc = "The user's associated quote mint token account.")]
    #[account(4, writable, name = "base_market_ata",     desc = "The market's associated base mint token account.")]
    #[account(5, writable, name = "quote_market_ata",    desc = "The market's associated quote mint token account.")]
    #[account(6,           name = "base_mint",           desc = "The base token mint account.")]
    #[account(7,           name = "quote_mint",          desc = "The quote token mint account.")]
    #[account(8,           name = "base_token_program",  desc = "The base mint's token program.")]
    #[account(9,           name = "quote_token_program", desc = "The quote mint's token program.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    CloseSeat,


    #[account(0, signer,   name = "user",           desc = "The user depositing or registering their seat.")]
    #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
    #[account(2, writable, name = "user_ata",       desc = "The user's associated token account.")]
    #[account(3, writable, name = "market_ata",     desc = "The market's associated token account.")]
    #[account(4,           name = "mint",           desc = "The token mint account.")]
    #[account(5,           name = "token_program",  desc = "The mint's token program.")]
    #[args(trader: Address, "The trader's address.")]
    #[args(amount: u64, "The amount deposited.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]
    Deposit,

    #[account(0, signer,   name = "user",           desc = "The user depositing or registering their seat.")]
    #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
    #[account(2, writable, name = "user_ata",       desc = "The user's associated token account.")]
    #[account(3, writable, name = "market_ata",     desc = "The market's associated token account.")]
    #[account(4,           name = "mint",           desc = "The token mint account.")]
    #[account(5,           name = "token_program",  desc = "The mint's token program.")]
    #[args(trader: Address, "The trader's address.")]
    #[args(amount: u64, "The amount withdrawn.")]
    #[args(is_base: bool, "Which token, i.e., `true` => base token, `false` => quote token.")]

    Withdraw,

    #[account(0, signer, name = "user")]
    #[args(big_info_1: BigOrderInfo, "Big order info 1.")]
    #[args(big_info_2: BigOrderInfo, "Big order info 2.")]
    #[args(big_info_3: BigOrderInfo, "Big order info 3.")]
    BigOrderInfos,

    #[account(0, signer, name = "event_authority", desc = "Flush events.")]
    FlushEvents,

    Batch,
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

    crate::create_big_order_info_test!();
}
