//! See [`MarketHeader`].

use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        sector::{
            LeSectorIndex,
            SectorIndex,
            NIL,
        },
        transmutable::Transmutable,
        LeU32,
        LeU64,
        U32_SIZE,
        U64_SIZE,
    },
};

pub const MARKET_ACCOUNT_DISCRIMINANT: u64 = 0xd00d00b00b00f00du64;

/// The lightweight header for each market account. This header contains metadata used to interpret
/// a market's account data properly.
///
/// A market account’s data consists of a statically sized [`MarketHeader`] followed by its
/// dynamically sized `sectors` region stored as raw bytes: `&[u8]`.
///
/// The metadata stored in the market header is central to interpreting the structures contained
/// within the market’s `sectors` bytes. This region acts as an untagged union of data structures
/// that share a common iterable layout, where each `Item` is a node with some payload type `T`.
///
/// For example, [`MarketHeader::free_stack_top`] exposes the index of the top node in the free
/// stack, allowing traversal of all available sectors. The payload type `T` in this case is
/// [`crate::state::free_stack::FreeNodePayload`].
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MarketHeader {
    /// The u64 market account's account discriminant as LE bytes.
    discriminant: LeU64,
    /// The u32 total number of fully initialized seats as LE bytes.
    num_seats: LeU32,
    /// The u32 total number of sectors in the free stack as LE bytes.
    num_free_sectors: LeU32,
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
    const LEN: usize = 104;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(MarketHeader::LEN, size_of::<MarketHeader>());
const_assert_eq!(align_of::<MarketHeader>(), 1);

impl MarketHeader {
    /// Initializes market header data to the header destination pointer with a `core::ptr::write`.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - `header_dst_ptr` points to allocated memory with at least `MarketHeader::LEN` bytes.
    /// - The pointer has exclusive mutable access (no active borrows or aliases)
    #[inline(always)]
    pub unsafe fn init(
        header_dst_ptr: *mut MarketHeader,
        market_bump: u8,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) {
        let header = MarketHeader {
            discriminant: MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes(),
            num_seats: [0; U32_SIZE],
            num_free_sectors: [0; U32_SIZE],
            free_stack_top: NIL.to_le_bytes(),
            seat_dll_head: NIL.to_le_bytes(),
            seat_dll_tail: NIL.to_le_bytes(),
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            market_bump,
            nonce: [0; U64_SIZE],
            _padding: [0; 3],
        };
        core::ptr::write(header_dst_ptr, header);
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
        u32::from_le_bytes(self.free_stack_top)
    }

    #[inline(always)]
    pub fn set_free_stack_top(&mut self, index: SectorIndex) {
        self.free_stack_top = index.to_le_bytes();
    }

    #[inline(always)]
    pub fn seat_dll_head(&self) -> SectorIndex {
        u32::from_le_bytes(self.seat_dll_head)
    }

    #[inline(always)]
    pub fn set_seat_dll_head(&mut self, index: SectorIndex) {
        self.seat_dll_head = index.to_le_bytes();
    }

    #[inline(always)]
    pub fn seat_dll_tail(&self) -> SectorIndex {
        u32::from_le_bytes(self.seat_dll_tail)
    }

    #[inline(always)]
    pub fn set_seat_dll_tail(&mut self, index: SectorIndex) {
        self.seat_dll_tail = index.to_le_bytes();
    }

    #[inline(always)]
    pub fn nonce(&self) -> u64 {
        u64::from_le_bytes(self.nonce)
    }

    #[inline(always)]
    pub fn increment_nonce(&mut self) {
        self.nonce = (self.nonce().saturating_add(1)).to_le_bytes();
    }
}
