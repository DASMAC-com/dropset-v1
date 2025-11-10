//! PDA helpers for deriving `dropset` program addresses.

use solana_sdk::pubkey::Pubkey;

pub fn find_market_address(base_mint: &Pubkey, quote_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            base_mint.as_ref(),
            quote_mint.as_ref(),
            dropset::MARKET_SEED_STR,
        ],
        &dropset::ID.into(),
    )
}
