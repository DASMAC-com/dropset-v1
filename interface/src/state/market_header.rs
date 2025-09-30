use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        sector::{LeSectorIndex, SectorIndex, NIL_LE},
        transmutable::Transmutable,
        U32_SIZE,
    },
};

pub const MARKET_HEADER_SIZE: usize = 96;
pub const MARKET_ACCOUNT_DISCRIMINANT: u64 = 0xd00d00b00b00f00du64;

const_assert_eq!(MARKET_HEADER_SIZE, size_of::<MarketHeader>());

#[repr(C)]
#[derive(Clone, Debug)]
pub struct MarketHeader {
    discriminant: [u8; 8],
    len: [u8; U32_SIZE],
    free_head: LeSectorIndex,
    deque_head: LeSectorIndex,
    deque_tail: LeSectorIndex,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    pub version: u8,
    pub market_bump: u8,
    // Ensure alignment 8 for the data that comes after header.
    _padding: [u8; 6],
}

unsafe impl Transmutable for MarketHeader {
    const LEN: usize = MARKET_HEADER_SIZE;
}

impl MarketHeader {
    pub fn init(market_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        MarketHeader {
            discriminant: MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes(),
            len: [0; U32_SIZE],
            free_head: NIL_LE,
            deque_head: NIL_LE,
            deque_tail: NIL_LE,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            version: 0,
            market_bump,
            _padding: [0; 6],
        }
    }

    #[inline(always)]
    pub fn verify_discriminant(&self) -> DropsetResult {
        if self.discriminant() != MARKET_ACCOUNT_DISCRIMINANT {
            return Err(DropsetError::InvalidAccountDiscriminant);
        }
        Ok(())
    }

    #[inline(always)]
    pub fn discriminant(&self) -> u64 {
        u64::from_le_bytes(self.discriminant)
    }

    #[inline(always)]
    fn len(&self) -> u32 {
        u32::from_le_bytes(self.len)
    }

    #[inline(always)]
    fn set_len(&mut self, amount: u32) {
        self.len = amount.to_le_bytes();
    }

    #[inline(always)]
    fn free_head(&self) -> SectorIndex {
        self.free_head.get()
    }

    #[inline(always)]
    fn set_free_head(&mut self, index: SectorIndex) {
        self.free_head.set(index);
    }

    #[inline(always)]
    fn deque_head(&self) -> SectorIndex {
        self.deque_head.get()
    }

    #[inline(always)]
    fn set_deque_head(&mut self, index: SectorIndex) {
        self.deque_head.set(index);
    }

    #[inline(always)]
    fn deque_tail(&self) -> SectorIndex {
        self.deque_tail.get()
    }

    #[inline(always)]
    fn set_deque_tail(&mut self, index: SectorIndex) {
        self.deque_tail.set(index);
    }
}
