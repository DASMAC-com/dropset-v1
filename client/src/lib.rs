use solana_sdk::pubkey::Pubkey;

pub mod context;
pub mod logs;
pub mod pda;
pub mod pretty;
pub mod test_accounts;
pub mod transaction_parser;
pub mod transactions;
pub mod views;

pub use logs::LogColor;

pub const SPL_TOKEN_ID: [u8; 32] = *spl_token_interface::ID.as_array();
pub const SPL_TOKEN_2022_ID: [u8; 32] = *spl_token_2022_interface::ID.as_array();
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_ID: [u8; 32] =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").to_bytes();
pub const COMPUTE_BUDGET_ID: [u8; 32] =
    Pubkey::from_str_const("ComputeBudget111111111111111111111111111111").to_bytes();
