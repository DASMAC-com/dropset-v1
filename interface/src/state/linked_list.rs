use pinocchio::pubkey::{pubkey_eq, Pubkey};

use crate::{
    error::DropsetError,
    pack::Pack,
    state::{
        free_stack::Stack,
        market_header::MarketHeader,
        market_seat::MarketSeat,
        node::{Node, NODE_PAYLOAD_SIZE},
        sector::{NonNilSectorIndex, SectorIndex, NIL},
    },
};

/// A sorted, doubly linked list.
#[derive(Debug)]
pub struct LinkedList<'a> {
    header: &'a mut MarketHeader,
    sectors: &'a mut [u8],
}

impl<'a> LinkedList<'a> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        LinkedList { header, sectors }
    }

    /// Helper method to pop a node from the free stack. Returns the node's sector index.
    fn acquire_free_node(&mut self) -> Result<NonNilSectorIndex, DropsetError> {
        let mut free_stack =
            Stack::new_from_parts(self.header.free_stack_top_mut_ref(), self.sectors);
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

    pub fn iter_seats(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: self.header.seat_dll_head(),
            sectors: self.sectors,
        }
    }

    pub fn insert_market_seat(
        &mut self,
        seat: MarketSeat,
    ) -> Result<NonNilSectorIndex, DropsetError> {
        let insert_index = self.find_insert_index(&seat.user);
        // Safety: MarketSeat adheres to all layout, alignment, and size constraints.
        let seat_bytes = unsafe { seat.as_bytes() };
        match insert_index {
            SectorIndex(0) => self.push_front(seat_bytes),
            NIL => self.push_back(seat_bytes),
            i => self.insert_before(NonNilSectorIndex::new_unchecked(i), seat_bytes),
        }
    }

    /// Find a node given an index hint.
    ///
    /// Returns an Err if the hint provided is invalid.
    pub fn find_node_with_hint(
        &mut self,
        hint: NonNilSectorIndex,
        user: &Pubkey,
    ) -> Result<&mut Node, DropsetError> {
        let node = Node::from_non_nil_sector_index_mut(self.sectors, hint)?;
        let seat = node.load_payload_mut::<MarketSeat>();
        if pubkey_eq(user, &seat.user) {
            Ok(node)
        } else {
            Err(DropsetError::InvalidIndexHint)
        }
    }

    /// Returns the index a node should be inserted before.
    ///
    /// ### NOTE: This function does not check for duplicates.
    /// This function does not check for the user already being registered in the seat
    /// list. This *will* insert duplicates without prior checks!
    ///
    /// - `0` => Insert at the front of the list
    /// - `1..n` => Insert at `n - 1`, where `n` is an in-bounds index
    /// - `NIL` => Insert at the end of the list
    pub fn find_insert_index(&self, user: &Pubkey) -> SectorIndex {
        for (index, node) in self.iter_seats() {
            let seat = node.load_payload::<MarketSeat>();
            // A user that already exists in the seat list should never be passed.
            debug_assert_ne!(user, &seat.user);
            if user < &seat.user {
                // At 0, this inserts at front.
                return index.get();
            }
        }
        // Insert at back.
        NIL
    }
}

pub struct LinkedListIter<'a> {
    curr: SectorIndex,
    sectors: &'a [u8],
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
