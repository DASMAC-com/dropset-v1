use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        market_header::MarketHeader,
        node::{Node, NodePayload, NODE_PAYLOAD_SIZE},
        sector::{LeSectorIndex, NonNilSectorIndex, SectorIndex},
        transmutable::Transmutable,
        U32_SIZE,
    },
};

pub struct Stack<'a> {
    header: &'a mut MarketHeader,
    /// A mutable reference to the sector index as LE bytes for the node at the top of the stack.
    // top: &'a mut LeSectorIndex,
    /// A mutable reference to the total number of free sectors in the stack as LE bytes.
    // num_free_sectors: &'a mut [u8; U32_SIZE],
    /// The slab of bytes where a Stack of FreeNodePayload exists, where sectors are untagged unions
    /// of (any possible Market account data type | FreeNodePayload).
    sectors: &'a mut [u8],
}

#[repr(transparent)]
pub struct FreeNodePayload(pub [u8; NODE_PAYLOAD_SIZE]);

unsafe impl Transmutable for FreeNodePayload {
    const LEN: usize = NODE_PAYLOAD_SIZE;
}

impl NodePayload for FreeNodePayload {}

impl<'a> Stack<'a> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        Stack { header, sectors }
    }

    /// Initialize zeroed out bytes as free stack nodes. This method avoids costly operations by
    /// making several assumptions mentioned in the safety contract below.
    ///
    /// # Safety
    /// Caller guarantees:
    /// - Account data from sector index `start` to `end` consists entirely of zeroed out bytes.
    /// - `start < end`
    /// - `end` is in-bounds of the account's data.
    /// - `start` and `end` are both non-NIL.
    pub unsafe fn push_free_nodes(&mut self, start: u32, end: u32) -> DropsetResult {
        debug_assert!(start < end);

        for i in (start..end).rev().map(SectorIndex) {
            let curr_top = self.top();

            // Safety: caller guarantees the safety contract for this method.
            let node = unsafe { Node::from_sector_index_mut_unchecked(self.sectors, i) };

            debug_assert_eq!(
                node.load_payload::<FreeNodePayload>().0,
                [0u8; NODE_PAYLOAD_SIZE]
            );

            node.set_next(curr_top);
            self.set_top(i);
            self.increment_num_free_sectors();
        }

        Ok(())
    }

    pub fn remove_free_node(&mut self) -> Result<NonNilSectorIndex, DropsetError> {
        if self.top().is_nil() {
            return Err(DropsetError::NoFreeNodesLeft);
        }

        // The free node is the node at the top of the stack.
        let free_index = self.top();
        let node_being_freed = Node::from_sector_index_mut(self.sectors, free_index)?;
        // Zero out the rest of the node by setting `next` to 0. The payload and `prev` were zeroed
        // out when adding to the free list.
        node_being_freed.set_next(SectorIndex(0));

        // Set the new `top` to the current top's `next`.
        let new_top = node_being_freed.next();
        self.set_top(new_top);
        self.decrement_num_free_sectors();

        // And return the index of the freed node.
        Ok(NonNilSectorIndex::new_unchecked(free_index))
    }

    #[inline(always)]
    pub fn top(&self) -> SectorIndex {
        self.header.free_stack_top()
    }

    #[inline(always)]
    pub fn set_top(&mut self, index: SectorIndex) {
        self.header.free_stack_top_mut_ref().set(index);
    }

    #[inline(always)]
    pub fn num_free_sectors(&self) -> u32 {
        self.header.num_free_sectors()
    }

    #[inline(always)]
    pub fn increment_num_free_sectors(&mut self) {
        *self.header.num_free_sectors_mut_ref() =
            self.num_free_sectors().saturating_add(1).to_le_bytes();
    }

    #[inline(always)]
    pub fn decrement_num_free_sectors(&mut self) {
        *self.header.num_free_sectors_mut_ref() =
            self.num_free_sectors().saturating_sub(1).to_le_bytes();
    }
}
