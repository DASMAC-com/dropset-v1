use static_assertions::const_assert_eq;

use crate::{
    error::DropsetError,
    state::{
        sector::{LeSectorIndex, SectorIndex},
        transmutable::Transmutable,
    },
};

pub const NODE_PAYLOAD_SIZE: usize = 48;

#[repr(C)]
pub struct Node {
    /// The little endian bytes representing the physical sector index of `prev`.
    /// Sector indexes map directly to the byte offset in memory, where the exact offset is the
    /// index multiplied by the size of the node in bytes.
    prev: LeSectorIndex,
    /// The little endian bytes representing the physical sector index of `next`.
    /// Sector indexes map directly to the byte offset in memory, where the exact offset is the
    /// index multiplied by the size of the node in bytes.
    next: LeSectorIndex,
    /// Either an in-use `MarketEscrow` or zeroed bytes.
    payload: [u8; NODE_PAYLOAD_SIZE],
}

/// Market trait to indicate that the type can be stored in the payload of a `Node`.
pub trait NodePayload: Transmutable {}

unsafe impl Transmutable for Node {
    const LEN: usize = 56;
}

// This check guarantees raw pointer dereferences to `Node` are always aligned.
const_assert_eq!(align_of::<Node>(), 1);

const_assert_eq!(core::mem::size_of::<Node>(), Node::LEN);

impl Node {
    #[inline(always)]
    pub fn prev(&self) -> SectorIndex {
        self.prev.get()
    }

    #[inline(always)]
    pub fn set_prev(&mut self, amount: SectorIndex) {
        self.prev.set(amount)
    }

    #[inline(always)]
    pub fn next(&self) -> SectorIndex {
        self.next.get()
    }

    #[inline(always)]
    pub fn set_next(&mut self, amount: SectorIndex) {
        self.next.set(amount)
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
        let i = usize::from(index.0);
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
    /// # Safety
    /// - Caller guarantees `index` has been verified as not `NIL`
    /// - Caller guarantees `index * Self::LEN` is within the bounds of `sectors` bytes
    pub unsafe fn from_sector_index_mut_unchecked(
        sectors: &mut [u8],
        index: SectorIndex,
    ) -> &mut Self {
        let i = usize::from(index.0);
        let byte_offset = i * Self::LEN;
        // Safety:
        // - Caller guarantees the sector index is in-bounds.
        // - The elided lifetime of `sectors` is tied to the reference this function returns.
        unsafe { &mut *(sectors.as_mut_ptr().add(byte_offset) as *mut Node) }
    }
}
