use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        market_header::{MarketHeader, MARKET_HEADER_SIZE}, node::NODE_PAYLOAD_SIZE, sector::SECTOR_SIZE, transmutable::{load_mut, load_mut_unchecked}, U64_SIZE
    },
};

#[repr(C)]
pub struct MarketEscrow {
    pub trader: Pubkey,
    base: [u8; U64_SIZE],
    quote: [u8; U64_SIZE],
}

#[repr(C)]
pub struct Market {
    pub header: MarketHeader,
    pub sectors: [u8],
}

const_assert_eq!(core::mem::size_of::<MarketEscrow>(), NODE_PAYLOAD_SIZE);

impl Market {
    pub fn init(
        uninitialized_market_account: &MarketAccountInfo,
        initial_num_sectors: u16,
        market_bump: u8,
        base_mint: MintAccountInfo,
        quote_mint: MintAccountInfo,
    ) -> DropsetResult {
        if uninitialized_market_account.len() < MARKET_HEADER_SIZE {
            return Err(DropsetError::UnallocatedAccountData);
        }

        let market = 
    }

    pub fn from_bytes_unchecked(data: &'a mut [u8]) -> Result<&mut Self, DropsetError> {
        let (header_bytes, sector_bytes) = data.split_at_mut(MARKET_HEADER_SIZE);
        let header = load_mut::<MarketHeader>(header_bytes)?;
        Ok(&mut Self {
            header, 
            sectors: sector_bytes,
        })
    }
}
