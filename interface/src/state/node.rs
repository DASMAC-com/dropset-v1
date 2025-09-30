use static_assertions::const_assert_eq;

use crate::state::{
    sector::{LeSectorIndex, SectorIndex},
    transmutable::Transmutable,
};

pub const NODE_PAYLOAD_SIZE: usize = 48;

#[repr(C)]
pub struct Node {
    /// The little endian bytes representing the physical sector index of `prev`. Under the hood,
    /// this represents a u32 as the raw byte offset of the previous `Node`.
    prev: LeSectorIndex,
    /// The little endian bytes representing the physical sector index of `prev`. Under the hood,
    /// this represents a u32 as the raw byte offset of the next `Node`.
    next: LeSectorIndex,
    /// Either an in-use `MarketEscrow` or zeroed bytes.
    payload: [u8; NODE_PAYLOAD_SIZE],
}

/// Market trait to indicate that the type can be stored in the payload of a `Node`.
pub trait NodePayload: Transmutable {}

unsafe impl Transmutable for Node {
    const LEN: usize = 56;
}

impl Node {
    #[inline(always)]
    fn prev(&self) -> SectorIndex {
        self.prev.get()
    }

    #[inline(always)]
    fn set_prev(&mut self, amount: SectorIndex) {
        self.prev.set(amount)
    }

    #[inline(always)]
    fn next(&self) -> SectorIndex {
        self.next.get()
    }

    #[inline(always)]
    fn set_next(&mut self, amount: SectorIndex) {
        self.next.set(amount)
    }
}

const_assert_eq!(core::mem::size_of::<Node>(), Node::LEN);
