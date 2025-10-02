use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        node::{Node, NodePayload, NODE_PAYLOAD_SIZE},
        sector::{LeSectorIndex, NonNilSectorIndex, SectorIndex},
        transmutable::Transmutable,
    },
};

pub struct Stack<'a> {
    /// The reference to the sector index as LE bytes for the node at the top of the stack.
    top: &'a mut LeSectorIndex,
    sectors: &'a mut [u8],
}

#[repr(transparent)]
pub struct FreeNodePayload(pub [u8; NODE_PAYLOAD_SIZE]);

unsafe impl Transmutable for FreeNodePayload {
    const LEN: usize = NODE_PAYLOAD_SIZE;
}

impl NodePayload for FreeNodePayload {}

impl<'a> Stack<'a> {
    pub fn new_from_parts(top: &'a mut LeSectorIndex, sectors: &'a mut [u8]) -> Self {
        Stack { top, sectors }
    }

    pub fn push_free_node(&mut self, index: SectorIndex) -> DropsetResult {
        let node = Node::from_sector_index_mut(self.sectors, index)?;
        node.zero_out_payload();

        // This isn't strictly necessary since `prev` should never be used in a free stack node
        // but it's a cheap call and makes the data cleaner/simpler.
        node.set_prev(SectorIndex(0));

        // Set the `next` node to `top`, and `top` to `index`.
        node.set_next(self.top.get());
        self.set_top(index);

        Ok(())
    }

    pub fn remove_free_node(&mut self) -> Result<NonNilSectorIndex, DropsetError> {
        if self.top().is_nil() {
            return Err(DropsetError::NoFreeNodesLeft);
        }

        // The free node is the node at the top of the stack.
        let free_index = self.top.get();
        let node_being_freed = Node::from_sector_index_mut(self.sectors, free_index)?;

        // Set the new `top` to the current top's `next`.
        self.top.set(node_being_freed.next());

        // Zero out the rest of the node by setting `next` to 0. The payload and `prev` were zeroed
        // out when adding to the free list.
        node_being_freed.set_next(SectorIndex(0));

        // And return the index of the freed node.
        Ok(NonNilSectorIndex::new_unchecked(free_index))
    }

    pub fn top(&self) -> SectorIndex {
        self.top.get()
    }

    pub fn set_top(&mut self, index: SectorIndex) {
        self.top.set(index);
    }
}
