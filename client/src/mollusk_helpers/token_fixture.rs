use solana_address::Address;
use transaction_parser::program_ids::SPL_TOKEN_ID;

pub struct TokenFixture {
    pub mint_address: Address,
    pub token_program: Address,
    pub mint_decimals: u8,
}

pub const DEFAULT_MINT_DECIMALS: u8 = 8;

impl Default for TokenFixture {
    fn default() -> Self {
        Self {
            mint_address: Address::new_unique(),
            token_program: SPL_TOKEN_ID,
            mint_decimals: DEFAULT_MINT_DECIMALS,
        }
    }
}

impl TokenFixture {
    pub fn new(mint_address: Address, token_program: Address, mint_decimals: u8) -> Self {
        Self {
            mint_address,
            token_program,
            mint_decimals,
        }
    }
}
