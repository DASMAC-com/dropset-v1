use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        free_stack::Stack,
        market::{MarketRef, MarketRefMut},
        market_header::MarketHeader,
        node::{Node, NODE_PAYLOAD_SIZE},
        sector::{NonNilSectorIndex, SectorIndex, NIL},
    },
};

/// A sorted, doubly linked list.
#[derive(Debug)]
pub struct LinkedList<'a> {
    pub header: &'a mut MarketHeader,
    pub sectors: &'a mut [u8],
}

impl<'a> LinkedList<'a> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        LinkedList { header, sectors }
    }

    /// Helper method to pop a node from the free stack. Returns the node's sector index.
    fn acquire_free_node(&mut self) -> Result<NonNilSectorIndex, DropsetError> {
        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.remove_free_node()
    }

    pub fn push_front(
        &mut self,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<NonNilSectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;
        let head_index = self.header.seat_dll_head();

        // Create the new node with the incoming payload. It has no `prev` and its `next` node is
        // the current head.
        let new_node = Node::from_non_nil_sector_index_mut(self.sectors, new_index)?;
        new_node.set_payload(payload);
        new_node.set_prev(NIL);
        new_node.set_next(head_index);

        if let Ok(head_index) = NonNilSectorIndex::new(head_index) {
            // If the head is a non-NIL sector index, set its `prev` to the new head index.
            Node::from_non_nil_sector_index_mut(self.sectors, head_index)?
                .set_prev(new_index.get());
        } else {
            // If the head is NIL, the new node is the only node and is thus also the tail.
            self.header.set_seat_dll_tail(new_index.get());
        }

        // Update the head to the new index and increment the number of seats.
        self.header.set_seat_dll_head(new_index.get());
        self.header.increment_num_seats();

        Ok(new_index)
    }

    pub fn push_back(
        &mut self,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<NonNilSectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;
        let tail_index = self.header.seat_dll_tail();

        // Create the new node with the incoming payload. It has no `next` and its `prev` node is
        // the current tail.
        let new_node = Node::from_non_nil_sector_index_mut(self.sectors, new_index)?;
        new_node.set_payload(payload);
        new_node.set_prev(tail_index);
        new_node.set_next(NIL);

        if let Ok(tail_index) = NonNilSectorIndex::new(tail_index) {
            // If the tail is a non-NIL sector index, set its `next` to the new tail index.
            Node::from_non_nil_sector_index_mut(self.sectors, tail_index)?
                .set_next(new_index.get());
        } else {
            // If the tail is NIL, the new node is the only node and is thus also the head.
            self.header.set_seat_dll_head(new_index.get());
        }

        // Update the tail to the new index and increment the number of seats.
        self.header.set_seat_dll_tail(new_index.get());
        self.header.increment_num_seats();

        Ok(new_index)
    }

    pub fn insert_before(
        &mut self,
        // The sector index of the node to insert a new node before.
        next_index: NonNilSectorIndex,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<NonNilSectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;

        // Store the next node's `prev` index.
        let next_node = Node::from_non_nil_sector_index_mut(self.sectors, next_index)?;
        let prev_index = next_node.prev();
        // Set `next_node`'s `prev` to the new node.
        next_node.set_prev(new_index.get());

        // Create the new node.
        let new_node = Node::from_non_nil_sector_index_mut(self.sectors, new_index)?;
        new_node.set_prev(prev_index);
        new_node.set_next(next_index.get());
        new_node.set_payload(payload);

        if let Ok(prev_index) = NonNilSectorIndex::new(prev_index) {
            // If `prev_index` is non-NIL, set it's `next` to the new index.
            Node::from_non_nil_sector_index_mut(self.sectors, prev_index)?
                .set_next(new_index.get());
        } else {
            // If `prev_index` is NIL, that means `next_index` was the head prior to this insertion,
            // and the head needs to be updated to the new node's index.
            self.header.set_seat_dll_head(new_index.get());
        }

        self.header.increment_num_seats();

        Ok(new_index)
    }

    /// Removes the node at the non-NIL sector `index` without checking the index validity.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - `index` is non-NIL.
    /// - `index` is in-bounds of the `sectors` bytes.
    pub unsafe fn remove_at(&mut self, index: NonNilSectorIndex) -> DropsetResult {
        let (prev_index, next_index) = {
            // Safety: Caller guarantees `index` is non-NIL and in-bounds.
            let node = unsafe { Node::from_sector_index_mut_unchecked(self.sectors, index.get()) };
            (node.prev(), node.next())
        };

        match prev_index {
            NIL => self.header.set_seat_dll_head(next_index),
            // Safety: `prev_index` matched against non-NIL and came from a node directly.
            prev_index => unsafe {
                Node::from_sector_index_mut_unchecked(self.sectors, prev_index)
                    .set_next(next_index);
            },
        }

        match next_index {
            NIL => self.header.set_seat_dll_tail(prev_index),
            // Safety: `next_index` matched against non-NIL and came from a node directly.
            next_index => unsafe {
                Node::from_sector_index_mut_unchecked(self.sectors, next_index)
                    .set_prev(prev_index);
            },
        }

        self.header.decrement_num_seats();

        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.push_free_node(index);

        Ok(())
    }

    pub fn iter(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: self.header.seat_dll_head(),
            sectors: self.sectors,
        }
    }
}

pub struct LinkedListIter<'a> {
    pub curr: SectorIndex,
    pub sectors: &'a [u8],
}

impl<'a> Iterator for LinkedListIter<'a> {
    type Item = (NonNilSectorIndex, &'a Node);

    fn next(&mut self) -> Option<(NonNilSectorIndex, &'a Node)> {
        if self.curr.is_nil() {
            return None;
        }

        let curr_non_nil = NonNilSectorIndex::new_unchecked(self.curr);
        let node = Node::from_non_nil_sector_index(self.sectors, curr_non_nil).ok()?;

        self.curr = node.next();
        Some((curr_non_nil, node))
    }
}

impl<'a> LinkedListIter<'a> {
    pub fn from_market(market: MarketRef<'a>) -> Self {
        LinkedListIter {
            curr: market.header.seat_dll_head(),
            sectors: market.sectors,
        }
    }

    pub fn from_market_mut(market: MarketRefMut<'a>) -> Self {
        LinkedListIter {
            curr: market.header.seat_dll_head(),
            sectors: market.sectors,
        }
    }
}
