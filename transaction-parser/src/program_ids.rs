//! Exports common program ID pubkeys as `[u8; 32]` arrays.

use solana_sdk::pubkey::Pubkey;

/// The SPL Token program ID as a `[u8; 32]`.
pub const SPL_TOKEN_ID: [u8; 32] =
    Pubkey::from_str_const("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").to_bytes();
/// The SPL Token 2022 program ID as a `[u8; 32]`.
pub const SPL_TOKEN_2022_ID: [u8; 32] =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb").to_bytes();
/// The SPL Associated Token Account program ID as a `[u8; 32]`.
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_ID: [u8; 32] =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").to_bytes();
/// The Solana Compute Budget program ID as a `[u8; 32]`.
pub const COMPUTE_BUDGET_ID: [u8; 32] =
    Pubkey::from_str_const("ComputeBudget111111111111111111111111111111").to_bytes();
