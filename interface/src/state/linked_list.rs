use crate::{
    error::DropsetError,
    state::{
        free_stack::Stack,
        market_header::MarketHeader,
        node::{
            Node,
            NODE_PAYLOAD_SIZE,
        },
        sector::{
            SectorIndex,
            NIL,
        },
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

    /// Helper method to pop a node from the free stack.
    ///
    /// A returned `Ok(index)` is always in-bounds and non-NIL.
    fn acquire_free_node(&mut self) -> Result<SectorIndex, DropsetError> {
        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.remove_free_node()
    }

    pub fn push_front(
        &mut self,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;
        let head_index = self.header.seat_dll_head();

        // Safety: `acquire_free_node` guarantees `new_index` is in-bounds and non-NIL.
        let new_node = unsafe { Node::from_sector_index_mut(self.sectors, new_index) };
        // Create the new node with the incoming payload. It has no `prev` and its `next` node is
        // the current head.
        new_node.set_payload(payload);
        new_node.set_prev(NIL);
        new_node.set_next(head_index);

        if head_index == NIL {
            // If the head is NIL, the new node is the only node and is thus also the tail.
            self.header.set_seat_dll_tail(new_index);
        } else {
            // Safety: `head_index` is non-NIL and per the linked list impl, must be in-bounds.
            let head = unsafe { Node::from_sector_index_mut(self.sectors, head_index) };
            // If the head is a non-NIL sector index, set its `prev` to the new head index.
            head.set_prev(new_index);
        }

        // Update the head to the new index and increment the number of seats.
        self.header.set_seat_dll_head(new_index);
        self.header.increment_num_seats();

        Ok(new_index)
    }

    pub fn push_back(
        &mut self,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;
        let tail_index = self.header.seat_dll_tail();

        // Safety: `acquire_free_node` guarantees `new_index` is in-bounds and non-NIL.
        let new_node = unsafe { Node::from_sector_index_mut(self.sectors, new_index) };
        // Create the new node with the incoming payload. It has no `next` and its `prev` node is
        // the current tail.
        new_node.set_payload(payload);
        new_node.set_prev(tail_index);
        new_node.set_next(NIL);

        if tail_index == NIL {
            // If the tail is NIL, the new node is the only node and is thus also the head.
            self.header.set_seat_dll_head(new_index);
        } else {
            // Safety: `tail_index` is non-NIL and per the linked list impl, must be in-bounds.
            let tail = unsafe { Node::from_sector_index_mut(self.sectors, tail_index) };
            // If the tail is a non-NIL sector index, set its `next` to the new tail index.
            tail.set_next(new_index);
        }

        // Update the tail to the new index and increment the number of seats.
        self.header.set_seat_dll_tail(new_index);
        self.header.increment_num_seats();

        Ok(new_index)
    }

    /// # Safety
    ///
    /// Caller must guarantee that `next_index` is in-bounds.
    pub unsafe fn insert_before(
        &mut self,
        // The sector index of the node to insert a new node before.
        next_index: SectorIndex,
        payload: &[u8; NODE_PAYLOAD_SIZE],
    ) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_node()?;

        // Safety: Caller must guarantee `next_index` is in-bounds.
        let next_node = unsafe { Node::from_sector_index_mut(self.sectors, next_index) };
        // Store the next node's `prev` index.
        let prev_index = next_node.prev();
        // Set `next_node`'s `prev` to the new node's index.
        next_node.set_prev(new_index);

        // Safety: `acquire_free_node` guarantees `new_index` is in-bounds.
        let new_node = unsafe { Node::from_sector_index_mut(self.sectors, new_index) };
        // Create the new node with the incoming payload, with its `prev` and `next` as the
        // corresponding adjacent nodes.
        new_node.set_prev(prev_index);
        new_node.set_next(next_index);
        new_node.set_payload(payload);

        if prev_index == NIL {
            // If `prev_index` is NIL, that means `next_index` was the head prior to this insertion,
            // so the `head` needs to be updated to the new node's index.
            self.header.set_seat_dll_head(new_index);
        } else {
            // Safety: `prev_index` is non-NIL and per the linked list impl, must be in-bounds.
            let prev = unsafe { Node::from_sector_index_mut(self.sectors, prev_index) };
            // If `prev_index` is non-NIL, set it's `next` to the new index.
            prev.set_next(new_index);
        }

        self.header.increment_num_seats();

        Ok(new_index)
    }

    /// Removes the node at the non-NIL sector `index` without checking the index validity.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index` is in-bounds.
    pub unsafe fn remove_at(&mut self, index: SectorIndex) {
        let (prev_index, next_index) = {
            // Safety: Caller guarantees `index` is in-bounds.
            let node = unsafe { Node::from_sector_index_mut(self.sectors, index) };
            (node.prev(), node.next())
        };

        match prev_index {
            NIL => self.header.set_seat_dll_head(next_index),
            // Safety: `prev_index` matched against non-NIL and came from a node directly.
            prev_index => unsafe {
                Node::from_sector_index_mut(self.sectors, prev_index).set_next(next_index);
            },
        }

        match next_index {
            NIL => self.header.set_seat_dll_tail(prev_index),
            // Safety: `next_index` matched against non-NIL and came from a node directly.
            next_index => unsafe {
                Node::from_sector_index_mut(self.sectors, next_index).set_prev(prev_index);
            },
        }

        self.header.decrement_num_seats();

        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.push_free_node(index);
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
    type Item = (SectorIndex, &'a Node);

    /// Returns the next node if it's non-NIL, otherwise, returns `None`.
    fn next(&mut self) -> Option<(SectorIndex, &'a Node)> {
        if self.curr == NIL {
            return None;
        }

        // Safety: `self.curr` is non-NIL and per the linked list impl, must be in-bounds.
        let node = unsafe { Node::from_sector_index(self.sectors, self.curr) };
        let res = (self.curr, node);

        self.curr = node.next();
        Some(res)
    }
}
