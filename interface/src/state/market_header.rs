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
    /// The market account's account discriminant, a u64 stored as little-endian bytes.
    discriminant: [u8; 8],
    /// The total number of fully initialized seats, a u32 stored as little-endian bytes.
    num_seats: [u8; U32_SIZE],
    /// The total number of sectors in the free stack.
    num_free_sectors: [u8; U32_SIZE],
    /// The sector index of the top (first) node in the stack of free nodes.
    free_stack_top: LeSectorIndex,
    /// The sector index of the head (first) node in the doubly linked list of seat nodes.
    seat_dll_head: LeSectorIndex,
    /// The sector index of the tail (last) node in the doubly linked list of seat nodes.
    seat_dll_tail: LeSectorIndex,
    /// The market's base mint public key.
    pub base_mint: Pubkey,
    /// The market's quote mint public key.
    pub quote_mint: Pubkey,
    /// The bump for the market PDA.
    pub market_bump: u8,
    // Ensure alignment 8 for the data that comes after header.
    _padding: [u8; 3],
}

unsafe impl Transmutable for MarketHeader {
    const LEN: usize = MARKET_HEADER_SIZE;
}

impl MarketHeader {
    pub fn init(market_bump: u8, base_mint: &Pubkey, quote_mint: &Pubkey) -> Self {
        MarketHeader {
            discriminant: MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes(),
            num_seats: [0; U32_SIZE],
            num_free_sectors: [0; U32_SIZE],
            free_stack_top: NIL_LE,
            seat_dll_head: NIL_LE,
            seat_dll_tail: NIL_LE,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            market_bump,
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
    pub fn num_free_sectors_mut_ref(&mut self) -> &mut [u8; U32_SIZE] {
        &mut self.num_free_sectors
    }

    #[inline(always)]
    pub fn free_stack_top_mut_ref(&mut self) -> &mut LeSectorIndex {
        &mut self.free_stack_top
    }

    #[inline(always)]
    pub fn free_stack_top(&self) -> SectorIndex {
        self.free_stack_top.get()
    }

    #[inline(always)]
    pub fn set_free_stack_top(&mut self, index: SectorIndex) {
        self.free_stack_top.set(index);
    }

    #[inline(always)]
    pub fn seat_dll_head(&self) -> SectorIndex {
        self.seat_dll_head.get()
    }

    #[inline(always)]
    pub fn set_seat_dll_head(&mut self, index: SectorIndex) {
        self.seat_dll_head.set(index);
    }

    #[inline(always)]
    pub fn seat_dll_tail(&self) -> SectorIndex {
        self.seat_dll_tail.get()
    }

    #[inline(always)]
    pub fn set_seat_dll_tail(&mut self, index: SectorIndex) {
        self.seat_dll_tail.set(index);
    }
}
