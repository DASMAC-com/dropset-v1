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
            SECTOR_SIZE,
        },
        transmutable::Transmutable,
    },
};

pub const NODE_PAYLOAD_SIZE: usize = 64;

#[repr(C)]
#[derive(Debug)]
/// An untagged union of Solana account data as a node in a tree, where `payload` is some type `T`
/// that implements `NodePayload`.
///
/// In the case of a free stack node, the payload is just zeroed bytes, with `next` pointing to the
/// sector index representing the memory address of the next free node.
pub struct Node {
    /// The little endian bytes representing the `next` node's sector index.
    next: LeSectorIndex,
    /// The little endian bytes representing the `prev` node's sector index.
    /// NOTE: This field is unused in the free stack node implementation and should be treated as
    /// random, meaningless bytes.
    prev: LeSectorIndex,
    /// The raw payload bytes for a Node, representing some type `T` that implements `NodePayload`.
    payload: [u8; NODE_PAYLOAD_SIZE],
}

/// Marker trait to indicate that the type can be stored in the payload of a `Node`.
///
/// # Safety
///
/// Implementor guarantees that `size_of::<T>() == NODE_PAYLOAD_SIZE` for some `T: NodePayload`.
pub unsafe trait NodePayload: Transmutable {}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for Node {
    const LEN: usize = SECTOR_SIZE;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(core::mem::size_of::<Node>(), Node::LEN);
const_assert_eq!(align_of::<Node>(), 1);

impl Node {
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
        unsafe { T::load_unchecked(&self.payload) }
    }

    #[inline(always)]
    pub fn load_payload_mut<T: NodePayload>(&mut self) -> &mut T {
        // Safety: All `NodePayload` implementations should have a length of `NODE_PAYLOAD_SIZE`.
        unsafe { T::load_unchecked_mut(&mut self.payload) }
    }

    #[inline(always)]
    pub fn check_in_bounds(sectors: &[u8], index: SectorIndex) -> DropsetResult {
        let max_num_sectors = (sectors.len() / Self::LEN) as u32;
        if index >= max_num_sectors {
            return Err(DropsetError::IndexOutOfBounds);
        };

        Ok(())
    }

    /// Convert a sector index to a Node without checking if the index is in-bounds.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index * Self::LEN` is within the bounds of `sectors` bytes.
    #[inline(always)]
    pub unsafe fn from_sector_index(sectors: &[u8], index: SectorIndex) -> &Self {
        let byte_offset = index as usize * Self::LEN;
        unsafe { &*(sectors.as_ptr().add(byte_offset) as *const Node) }
    }

    /// Convert a sector index to a mutable Node without checking if the index is in-bounds.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index * Self::LEN` is within the bounds of `sectors` bytes.
    #[inline(always)]
    pub unsafe fn from_sector_index_mut(sectors: &mut [u8], index: SectorIndex) -> &mut Self {
        let byte_offset = index as usize * Self::LEN;
        unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Node) }
    }
}
