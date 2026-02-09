//! Defines the size and sentinel constants for fixed-size storage sectors used to organize
//! and index market data efficiently in account memory.
//!
//! Also defines and implements [`Sector`] structs representing the fixed-size storage sector.

use pinocchio::hint::unlikely;
use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        market_seat::MarketSeat,
        transmutable::Transmutable,
        U32_SIZE,
    },
};

pub const SECTOR_SIZE: usize = 136;

/// A sentinel value that marks 1-past the last valid sector index.
///
/// This value will never appear naturally. Even at a sector size of 1 byte, Solana's max account
/// size of 10 MB would put the max sector index at ~10.5 mil — far less than u32::MAX.
pub const NIL: SectorIndex = u32::MAX;

/// The little-endian byte representation of [`NIL`].
pub const LE_NIL: LeSectorIndex = NIL.to_le_bytes();

// A sector index stored as little-endian bytes.
pub type LeSectorIndex = [u8; U32_SIZE];

/// A stride-based index into an array of sectors.
///
/// Index `i` maps to byte offset `i × SECTOR_SIZE` for a raw `sectors: &[u8]` slice.
pub type SectorIndex = u32;

/// The [`PAYLOAD_SIZE`] must equal the size of the largest data structure used as a payload.
pub const PAYLOAD_SIZE: usize = MarketSeat::LEN;

#[repr(C)]
#[derive(Debug)]
/// A [`Sector`] represents each fixed-size storage sector's byte layout as used in a market
/// account's `sectors` region for building linked structures, exposing previous/next indices and an
/// opaque payload segment.
///
/// Links are logical (by index), not physical adjacency.
///
/// A single sector stored in a market's sectors region, containing previous/next sector indices and
/// a fixed-size payload buffer.
///
/// Higher-level structures (such as free stacks or seat lists) interpret this payload as their own
/// logical type via [`Payload`] implementations.
pub struct Sector {
    /// The little endian bytes representing the [`SectorIndex`] of the `next` sector.
    next: LeSectorIndex,
    /// The little endian bytes representing the [`SectorIndex`] of the `prev` sector.
    ///
    /// This field is unused in the free stack implementation and should be treated as garbage data
    /// while a [`Sector`] is considered freed.
    prev: LeSectorIndex,
    /// The raw payload bytes for a [`Sector`], representing some type `T` that implements
    /// [`Payload`].
    payload: [u8; PAYLOAD_SIZE],
}

/// Marker trait to indicate that the type can be stored in a [`Sector::payload`].
///
/// # Safety
///
/// Implementor guarantees that `size_of::<T>() ==`[`PAYLOAD_SIZE`] for some `T:`
/// [`Payload`].
pub unsafe trait Payload: Transmutable {}

/// Marker trait to indicate that the type is valid for all bit patterns as long as the size
/// constraint is satisfied. It therefore doesn't require a check on individual bytes prior to
/// transmutation.
///
/// That is, it has no invalid enum variants, isn't a bool, etc.
///
/// # Safety
///
/// Implementor guarantees that all bit patterns are valid for some `T:`[`AllBitPatternsValid`].
pub unsafe trait AllBitPatternsValid: Transmutable {}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for Sector {
    const LEN: usize = SECTOR_SIZE;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(core::mem::size_of::<Sector>(), Sector::LEN);
const_assert_eq!(align_of::<Sector>(), 1);

impl Sector {
    #[inline(always)]
    pub fn prev(&self) -> SectorIndex {
        u32::from_le_bytes(self.prev)
    }

    #[inline(always)]
    pub fn set_prev(&mut self, index: SectorIndex) {
        self.prev = index.to_le_bytes();
    }

    #[inline(always)]
    pub fn next(&self) -> SectorIndex {
        u32::from_le_bytes(self.next)
    }

    #[inline(always)]
    pub fn set_next(&mut self, index: SectorIndex) {
        self.next = index.to_le_bytes();
    }

    #[inline(always)]
    pub fn set_payload(&mut self, payload: &[u8; PAYLOAD_SIZE]) {
        // Safety: both payloads are exactly `PAYLOAD_SIZE` long, and the incoming payload
        // should never overlap with the existing payload due to aliasing rules.
        unsafe {
            core::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                self.payload.as_mut_ptr(),
                PAYLOAD_SIZE,
            );
        }
    }

    #[inline(always)]
    pub fn zero_out_payload(&mut self) {
        // Safety: `payload` is exactly `PAYLOAD_SIZE` bytes long and align 1.
        unsafe {
            core::ptr::write_bytes(self.payload.as_mut_ptr(), 0, PAYLOAD_SIZE);
        }
    }

    #[inline(always)]
    pub fn load_payload<T: Payload + AllBitPatternsValid>(&self) -> &T {
        // Safety: All `Payload` implementations should have a length of `PAYLOAD_SIZE`.
        unsafe { T::load_unchecked(&self.payload) }
    }

    #[inline(always)]
    pub fn load_payload_mut<T: Payload + AllBitPatternsValid>(&mut self) -> &mut T {
        // Safety: All `Payload` implementations should have a length of `PAYLOAD_SIZE`.
        unsafe { T::load_unchecked_mut(&mut self.payload) }
    }

    /// Checks if a given sector index is in-bounds of the passed slice of sector bytes.
    #[inline(always)]
    pub fn check_in_bounds(sectors: &[u8], index: SectorIndex) -> DropsetResult {
        let max_num_sectors = (sectors.len() / Self::LEN) as u32;
        if unlikely(index >= max_num_sectors) {
            return Err(DropsetError::IndexOutOfBounds);
        };

        Ok(())
    }

    /// Convert a sector index to a [`Sector`] without checking if the index is in-bounds.
    ///
    /// # Safety
    ///
    /// Caller guarantees index * [`Sector::LEN`] is within the bounds of `sectors` bytes.
    #[inline(always)]
    pub unsafe fn from_sector_index(sectors: &[u8], index: SectorIndex) -> &Self {
        let byte_offset = index as usize * Self::LEN;
        unsafe { &*(sectors.as_ptr().add(byte_offset) as *const Sector) }
    }

    /// Convert a sector index to a mutable [`Sector`] without checking if the index is in-bounds.
    ///
    /// # Safety
    ///
    /// Caller guarantees index * [`Sector::LEN`] is within the bounds of `sectors` bytes.
    #[inline(always)]
    pub unsafe fn from_sector_index_mut(sectors: &mut [u8], index: SectorIndex) -> &mut Self {
        let byte_offset = index as usize * Self::LEN;
        unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Sector) }
    }
}
