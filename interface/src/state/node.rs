use static_assertions::const_assert_eq;

use crate::{
    error::DropsetError,
    state::{
        sector::{LeSectorIndex, NonNilSectorIndex, SectorIndex, SECTOR_SIZE},
        transmutable::{load_unchecked, load_unchecked_mut, Transmutable},
    },
};

pub const NODE_PAYLOAD_SIZE: usize = 64;

#[repr(C)]
#[derive(Debug)]
pub struct Node {
    /// The little endian bytes representing the physical sector index of `next`.
    /// Sector indexes map directly to the byte offset in memory, where the exact offset is the
    /// index multiplied by the size of the node in bytes.
    next: LeSectorIndex,
    /// The little endian bytes representing the physical sector index of `prev`.
    /// Sector indexes map directly to the byte offset in memory, where the exact offset is the
    /// index multiplied by the size of the node in bytes.
    /// NOTE: This field is entirely unused in the free stack of Nodes implementation and should be
    /// considered as random, meaningless bytes.
    prev: LeSectorIndex,
    /// Either an in-use [MarketSeat][crate::state::market_seat::MarketSeat] or zeroed bytes.
    payload: [u8; NODE_PAYLOAD_SIZE],
}

/// Marker trait to indicate that the type can be stored in the payload of a `Node`.
pub trait NodePayload: Transmutable {}

unsafe impl Transmutable for Node {
    const LEN: usize = SECTOR_SIZE;
}

const_assert_eq!(core::mem::size_of::<Node>(), Node::LEN);
const_assert_eq!(align_of::<Node>(), 1);

impl Node {
    #[inline(always)]
    pub fn prev(&self) -> SectorIndex {
        self.prev.get()
    }

    #[inline(always)]
    pub fn set_prev(&mut self, index: SectorIndex) {
        self.prev.set(index)
    }

    #[inline(always)]
    pub fn next(&self) -> SectorIndex {
        self.next.get()
    }

    #[inline(always)]
    pub fn set_next(&mut self, index: SectorIndex) {
        self.next.set(index)
    }

    #[inline(always)]
    pub fn set_payload(&mut self, payload: &[u8; NODE_PAYLOAD_SIZE]) {
        // Safety: both payloads are exactly `NODE_PAYLOAD_SIZE` long, and the incoming payload
        // should never overlap with the existing payload due to aliasing rules.
        unsafe {
            #[cfg(target_os = "solana")]
            pinocchio::syscalls::sol_memcpy_(
                self.payload.as_mut_ptr(),
                payload.as_ptr(),
                NODE_PAYLOAD_SIZE as u64,
            );

            #[cfg(not(target_os = "solana"))]
            core::ptr::copy_nonoverlapping(
                payload.as_ptr(),
                self.payload.as_mut_ptr(),
                NODE_PAYLOAD_SIZE,
            );
        }
    }

    #[inline(always)]
    pub fn zero_out_payload(&mut self) {
        // Safety: `payload` is exactly `NODE_PAYLOAD_SIZE` bytes long and align 1.
        unsafe {
            #[cfg(target_os = "solana")]
            pinocchio::syscalls::sol_memset_(
                self.payload.as_mut_ptr(),
                0,
                NODE_PAYLOAD_SIZE as u64,
            );

            #[cfg(not(target_os = "solana"))]
            core::ptr::write_bytes(self.payload.as_mut_ptr(), 0, NODE_PAYLOAD_SIZE);
        }
    }

    #[inline(always)]
    pub fn load_payload<T: NodePayload>(&self) -> &T {
        // Safety: All `NodePayload` implementations should have a length of `NODE_PAYLOAD_SIZE`.
        unsafe { load_unchecked::<T>(&self.payload) }
    }

    #[inline(always)]
    pub fn load_payload_mut<T: NodePayload>(&mut self) -> &mut T {
        // Safety: All `NodePayload` implementations should have a length of `NODE_PAYLOAD_SIZE`.
        unsafe { load_unchecked_mut::<T>(&mut self.payload) }
    }

    #[inline(always)]
    pub fn from_sector_index_mut(
        sectors: &mut [u8],
        index: SectorIndex,
    ) -> Result<&mut Self, DropsetError> {
        if index.is_nil() {
            return Err(DropsetError::InvalidSectorIndex);
        }

        let capacity = sectors.len() / Self::LEN;
        let i = index.0 as usize;
        if i >= capacity {
            return Err(DropsetError::IndexOutOfBounds);
        }

        let byte_offset = i * Self::LEN;

        // Safety:
        // - `byte_offset..byte_offset + Self::LEN` is in-bounds.
        // - The elided lifetime of `sectors` is tied to the reference this function returns.
        Ok(unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Node) })
    }

    #[inline(always)]
    pub fn from_non_nil_sector_index(
        sectors: &[u8],
        index: NonNilSectorIndex,
    ) -> Result<&Self, DropsetError> {
        let capacity = sectors.len() / Self::LEN;
        let i = index.get().0 as usize;
        if i >= capacity {
            return Err(DropsetError::IndexOutOfBounds);
        }

        let byte_offset = i * Self::LEN;
        // Safety: The index has been verified as not NIL, and in-bounds was just checked.
        Ok(unsafe { &*(sectors.as_ptr().add(byte_offset) as *const Node) })
    }

    #[inline(always)]
    pub fn from_non_nil_sector_index_mut(
        sectors: &mut [u8],
        index: NonNilSectorIndex,
    ) -> Result<&mut Self, DropsetError> {
        let capacity = sectors.len() / Self::LEN;
        let i = index.get().0 as usize;
        if i >= capacity {
            return Err(DropsetError::IndexOutOfBounds);
        }

        let byte_offset = i * Self::LEN;
        // Safety: The index has been verified as not NIL, and in-bounds was just checked.
        Ok(unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Node) })
    }

    #[inline(always)]
    /// # Safety
    /// - Caller guarantees `index` has been verified as not `NIL`
    /// - Caller guarantees `index * Self::LEN` is within the bounds of `sectors` bytes
    pub unsafe fn from_sector_index_mut_unchecked(
        sectors: &mut [u8],
        index: SectorIndex,
    ) -> &mut Self {
        let i = index.0 as usize;
        let byte_offset = i * Self::LEN;
        // Safety:
        // - Caller guarantees the sector index is in-bounds.
        // - The elided lifetime of `sectors` is tied to the reference this function returns.
        unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Node) }
    }
}
