use solana_address::Address;

use crate::mollusk_helpers::TokenFixture;

pub struct MarketFixture {
    pub market: Address,
    pub base: TokenFixture,
    pub quote: TokenFixture,
    pub base_market_ata: Address,
    pub quote_market_ata: Address,
}
