//! See [`Stack`].

use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        market_header::MarketHeader,
        node::{
            Node,
            NodePayload,
            NODE_PAYLOAD_SIZE,
        },
        sector::{
            SectorIndex,
            NIL,
        },
        transmutable::Transmutable,
    },
};

/// Implements a stack allocator abstraction for managing freed sectors and reusing space
/// efficiently.
pub struct Stack<'a> {
    /// See [`MarketHeader`].
    header: &'a mut MarketHeader,
    /// The slab of bytes where all sector data exists, where each sector is an untagged union
    /// of (any possible sector type | FreeNodePayload).
    sectors: &'a mut [u8],
}

#[repr(transparent)]
/// A free node payload is the unused payload portion of the "free" variant of the untagged union of
/// each sector type (seat node, order node, etc).
/// Since a free node only ever reads from the `next` field, it's not necessary to zero out the
/// payload bytes and thus they should be considered garbage data.
pub struct FreeNodePayload(pub [u8; NODE_PAYLOAD_SIZE]);

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for FreeNodePayload {
    const LEN: usize = NODE_PAYLOAD_SIZE;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(FreeNodePayload::LEN, size_of::<FreeNodePayload>());
const_assert_eq!(1, align_of::<FreeNodePayload>());

// Safety: FreeNodePayload's size is checked below.
unsafe impl NodePayload for FreeNodePayload {}

const_assert_eq!(size_of::<FreeNodePayload>(), NODE_PAYLOAD_SIZE);

impl<'a> Stack<'a> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        Stack { header, sectors }
    }

    /// Push a node at the sector index onto the stack as a free node by zeroing out its data,
    /// setting its `next` to the current `top`, and updating the stack `top`.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index` is in-bounds of the sector bytes.
    pub unsafe fn push_free_node(&mut self, index: SectorIndex) {
        let curr_top = self.top();

        let node = unsafe { Node::from_sector_index_mut(self.sectors, index) };
        node.zero_out_payload();

        node.set_next(curr_top);
        self.set_top(index);
    }

    /// Initialize zeroed out bytes as free stack nodes.
    ///
    /// This should only be called directly after increasing the size of the account data, since the
    /// account data's bytes in that case are always zero-initialized.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - Account data from sector index `start` to `end` is already zeroed out bytes.
    /// - `start < end`
    /// - `end` is in-bounds of the account's data.
    /// - `start` and `end` are both non-NIL.
    pub unsafe fn convert_zeroed_bytes_to_free_nodes(
        &mut self,
        start: u32,
        end: u32,
    ) -> DropsetResult {
        // Debug check that the node has been zeroed out.
        debug_assert!(
            start < end
                && (start..end).all(|i| {
                    // Safety: The safety contract guarantees the index is always in-bounds.
                    let node = unsafe { Node::from_sector_index_mut(self.sectors, i) };
                    node.load_payload::<FreeNodePayload>().0 == [0; NODE_PAYLOAD_SIZE]
                })
        );

        for index in (start..end).rev() {
            let curr_top = self.top();

            // Safety: The safety contract guarantees the index is always in-bounds.
            let node = unsafe { Node::from_sector_index_mut(self.sectors, index) };

            node.set_next(curr_top);
            self.set_top(index);
            self.header.increment_num_free_sectors();
        }

        Ok(())
    }

    /// Tries to remove a free node and if successful, returns its sector index.
    ///
    /// The sector index returned is always in-bounds and non-NIL.
    pub fn remove_free_node(&mut self) -> Result<SectorIndex, DropsetError> {
        if self.top() == NIL {
            return Err(DropsetError::NoFreeNodesLeft);
        }

        // The free node is the node at the top of the stack.
        let free_index = self.top();

        Node::check_in_bounds(self.sectors, free_index)?;
        // Safety: The free index was just checked as in-bounds.
        let node_being_freed = unsafe { Node::from_sector_index_mut(self.sectors, free_index) };

        // Copy the current top's `next` as that will become the new `top`.
        let new_top = node_being_freed.next();

        // Zero out the rest of the node by setting `next` to 0. The payload and `prev` were zeroed
        // out when adding to the free list.
        node_being_freed.set_next(0);

        self.set_top(new_top);
        self.header.decrement_num_free_sectors();

        // Now return the index of the freed node.
        Ok(free_index)
    }

    #[inline(always)]
    pub fn top(&self) -> SectorIndex {
        self.header.free_stack_top()
    }

    #[inline(always)]
    pub fn set_top(&mut self, index: SectorIndex) {
        self.header.set_free_stack_top(index);
    }
}
