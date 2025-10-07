use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        sector::{LeSectorIndex, SectorIndex, NIL},
        transmutable::Transmutable,
        LeU32, LeU64, U32_SIZE, U64_SIZE,
    },
};

pub const MARKET_HEADER_SIZE: usize = 104;
pub const MARKET_ACCOUNT_DISCRIMINANT: u64 = 0xd00d00b00b00f00du64;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct MarketHeader {
    /// The u64 market account's account discriminant as LE bytes.
    discriminant: LeU64,
    /// The u32 total number of fully initialized seats as LE bytes.
    num_seats: LeU32,
    /// The u32 total number of sectors in the free stack as LE bytes.
    num_free_sectors: LeSectorIndex,
    /// The u32 sector index of the first node in the stack of free nodes as LE bytes.
    free_stack_top: LeSectorIndex,
    /// The u32 sector index of the first node in the doubly linked list of seat nodes as LE bytes.
    seat_dll_head: LeSectorIndex,
    /// The u32 sector index of the last node in the doubly linked list of seat nodes as LE bytes.
    seat_dll_tail: LeSectorIndex,
    /// The market's base mint public key.
    pub base_mint: Pubkey,
    /// The market's quote mint public key.
    pub quote_mint: Pubkey,
    /// The bump for the market PDA.
    pub market_bump: u8,
    /// The u64 market nonce as LE bytes.
    nonce: LeU64,
    // Although not necessary, add extra padding to make this alignment 8.
    _padding: [u8; 3],
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for MarketHeader {
    const LEN: usize = MARKET_HEADER_SIZE;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(MARKET_HEADER_SIZE, size_of::<MarketHeader>());
const_assert_eq!(align_of::<MarketHeader>(), 1);

impl MarketHeader {
    pub fn init(market_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        MarketHeader {
            discriminant: MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes(),
            num_seats: [0; U32_SIZE],
            num_free_sectors: [0; U32_SIZE],
            free_stack_top: NIL.into(),
            seat_dll_head: NIL.into(),
            seat_dll_tail: NIL.into(),
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            market_bump,
            nonce: [0; U64_SIZE],
            _padding: [0; 3],
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
    pub fn num_seats(&self) -> u32 {
        u32::from_le_bytes(self.num_seats)
    }

    #[inline(always)]
    pub fn increment_num_seats(&mut self) {
        self.num_seats = self.num_seats().saturating_add(1).to_le_bytes();
    }

    #[inline(always)]
    pub fn decrement_num_seats(&mut self) {
        self.num_seats = self.num_seats().saturating_sub(1).to_le_bytes();
    }

    #[inline(always)]
    pub fn num_free_sectors(&self) -> u32 {
        u32::from_le_bytes(self.num_free_sectors)
    }

    #[inline(always)]
    pub fn increment_num_free_sectors(&mut self) {
        self.num_free_sectors = self.num_free_sectors().saturating_add(1).to_le_bytes();
    }

    #[inline(always)]
    pub fn decrement_num_free_sectors(&mut self) {
        self.num_free_sectors = self.num_free_sectors().saturating_sub(1).to_le_bytes();
    }

    #[inline(always)]
    pub fn free_stack_top(&self) -> SectorIndex {
        self.free_stack_top.into()
    }

    #[inline(always)]
    pub fn set_free_stack_top(&mut self, index: SectorIndex) {
        self.free_stack_top = index.into();
    }

    #[inline(always)]
    pub fn seat_dll_head(&self) -> SectorIndex {
        self.seat_dll_head.into()
    }

    #[inline(always)]
    pub fn set_seat_dll_head(&mut self, index: SectorIndex) {
        self.seat_dll_head = index.into();
    }

    #[inline(always)]
    pub fn seat_dll_tail(&self) -> SectorIndex {
        self.seat_dll_tail.into()
    }

    #[inline(always)]
    pub fn set_seat_dll_tail(&mut self, index: SectorIndex) {
        self.seat_dll_tail = index.into();
    }
}
