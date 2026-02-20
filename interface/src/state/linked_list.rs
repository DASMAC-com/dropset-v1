use core::marker::PhantomData;

use crate::{
    error::DropsetError,
    state::{
        free_stack::Stack,
        market_header::MarketHeader,
        sector::{
            Sector,
            SectorIndex,
            NIL,
            PAYLOAD_SIZE,
        },
    },
};

pub trait LinkedListHeaderOperations {
    fn head(header: &MarketHeader) -> SectorIndex;

    fn set_head(header: &mut MarketHeader, new_index: SectorIndex);

    fn tail(header: &MarketHeader) -> SectorIndex;

    fn set_tail(header: &mut MarketHeader, new_index: SectorIndex);

    fn increment_num_elements(header: &mut MarketHeader);

    fn decrement_num_elements(header: &mut MarketHeader);
}

/// A doubly linked list of sectors containing arbitrary payloads of size
/// [`crate::state::sector::PAYLOAD_SIZE`].
///
/// Each sector exists as a union of the traversable payload type and a free sector.
pub struct LinkedList<'a, T: LinkedListHeaderOperations> {
    pub header: &'a mut MarketHeader,
    pub sectors: &'a mut [u8],
    _list_type: PhantomData<T>,
}

impl<'a, T: LinkedListHeaderOperations> LinkedList<'a, T> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        Self {
            header,
            sectors,
            _list_type: PhantomData,
        }
    }

    /// Helper method to pop a sector from the free stack.
    ///
    /// An Ok([`SectorIndex`]) is always in-bounds and non-NIL.
    ///
    /// NOTE: See [`Stack::pop_free_sector`] for how to safely avoid bricking the freed sector. The
    /// sector's data is also not zeroed prior to return.
    #[inline(always)]
    fn acquire_free_sector(&mut self) -> Result<SectorIndex, DropsetError> {
        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.pop_free_sector()
    }

    pub fn push_front(
        &mut self,
        payload: &[u8; PAYLOAD_SIZE],
    ) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_sector()?;
        let head_index = T::head(self.header);

        // Safety: `acquire_free_sector` guarantees `new_index` is in-bounds and non-NIL.
        let new_sector = unsafe { Sector::from_sector_index_mut(self.sectors, new_index) };
        // Create the new sector with the incoming payload. It has no `prev` and its `next` sector
        // is the current head.
        new_sector.set_payload(payload);
        new_sector.set_prev(NIL);
        new_sector.set_next(head_index);

        if head_index == NIL {
            // If the head is NIL, the new sector is the only sector and is thus also the tail.
            T::set_tail(self.header, new_index);
        } else {
            // Safety: `head_index` is non-NIL and per the linked list impl, must be in-bounds.
            let head = unsafe { Sector::from_sector_index_mut(self.sectors, head_index) };
            // If the head is a non-NIL sector index, set its `prev` to the new head index.
            head.set_prev(new_index);
        }

        T::set_head(self.header, new_index);
        T::increment_num_elements(self.header);

        Ok(new_index)
    }

    pub fn push_back(&mut self, payload: &[u8; PAYLOAD_SIZE]) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_sector()?;
        let tail_index = T::tail(self.header);

        // Safety: `acquire_free_sector` guarantees `new_index` is in-bounds and non-NIL.
        let new_sector = unsafe { Sector::from_sector_index_mut(self.sectors, new_index) };
        // Create the new sector with the incoming payload. It has no `next` and its `prev` sector
        // is the current tail.
        new_sector.set_payload(payload);
        new_sector.set_prev(tail_index);
        new_sector.set_next(NIL);

        if tail_index == NIL {
            // If the tail is NIL, the new sector is the only sector and is thus also the head.
            T::set_head(self.header, new_index);
        } else {
            // Safety: `tail_index` is non-NIL and per the linked list impl, must be in-bounds.
            let tail = unsafe { Sector::from_sector_index_mut(self.sectors, tail_index) };
            // If the tail is a non-NIL sector index, set its `next` to the new tail index.
            tail.set_next(new_index);
        }

        T::set_tail(self.header, new_index);
        T::increment_num_elements(self.header);

        Ok(new_index)
    }

    /// # Safety
    ///
    /// Caller must guarantee that `next_index` is in-bounds.
    pub unsafe fn insert_before(
        &mut self,
        // The sector index of the sector to insert a new sector before.
        next_index: SectorIndex,
        payload: &[u8; PAYLOAD_SIZE],
    ) -> Result<SectorIndex, DropsetError> {
        let new_index = self.acquire_free_sector()?;

        // Safety: Caller must guarantee `next_index` is in-bounds.
        let next_sector = unsafe { Sector::from_sector_index_mut(self.sectors, next_index) };
        // Store the next sector's `prev` index.
        let prev_index = next_sector.prev();
        // Set `next_sector`'s `prev` to the new sector's index.
        next_sector.set_prev(new_index);

        // Safety: `acquire_free_sector` guarantees `new_index` is in-bounds.
        let new_sector = unsafe { Sector::from_sector_index_mut(self.sectors, new_index) };
        // Create the new sector with the incoming payload, with its `prev` and `next` as the
        // corresponding adjacent sectors.
        new_sector.set_prev(prev_index);
        new_sector.set_next(next_index);
        new_sector.set_payload(payload);

        if prev_index == NIL {
            // If `prev_index` is NIL, that means `next_index` was the head prior to this insertion,
            // so the `head` needs to be updated to the new sector's index.
            T::set_head(self.header, new_index);
        } else {
            // Safety: `prev_index` is non-NIL and per the linked list impl, must be in-bounds.
            let prev = unsafe { Sector::from_sector_index_mut(self.sectors, prev_index) };
            // If `prev_index` is non-NIL, set it's `next` to the new index.
            prev.set_next(new_index);
        }

        T::increment_num_elements(self.header);

        Ok(new_index)
    }

    /// Removes the sector at the non-NIL sector `index` without checking the index validity.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index` is in-bounds.
    pub unsafe fn remove_at(&mut self, index: SectorIndex) {
        let (prev_index, next_index) = {
            // Safety: Caller guarantees `index` is in-bounds.
            let sector = unsafe { Sector::from_sector_index_mut(self.sectors, index) };
            (sector.prev(), sector.next())
        };

        match prev_index {
            NIL => T::set_head(self.header, next_index),
            // Safety: `prev_index` matched against non-NIL and came from a sector directly.
            prev_index => unsafe {
                Sector::from_sector_index_mut(self.sectors, prev_index).set_next(next_index);
            },
        }

        match next_index {
            NIL => T::set_tail(self.header, prev_index),
            // Safety: `next_index` matched against non-NIL and came from a sector directly.
            next_index => unsafe {
                Sector::from_sector_index_mut(self.sectors, next_index).set_prev(prev_index);
            },
        }

        T::decrement_num_elements(self.header);

        let mut free_stack = Stack::new_from_parts(self.header, self.sectors);
        free_stack.push_free_sector(index);
    }

    pub fn iter(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: T::head(self.header),
            sectors: self.sectors,
        }
    }

    /// Creates an iterator starting from the specified sector index.
    ///
    /// Useful for batch operations where you want to continue iteration from a specific position
    /// rather than always starting from the head.
    ///
    /// # Safety
    ///
    /// Caller must guarantee that:
    /// - `start` is either NIL or a valid, in-bounds sector index.
    /// - `start` belongs to this specific `LinkedList<T>` instance (same collection type and
    ///   sectors).
    /// - The returned iterator will only be used with operations on this same `LinkedList<T>`
    ///   instance.
    ///
    /// Violating these invariants can lead to undefined behavior through inconsistent metadata
    /// (e.g., mixing a bid list iterator with ask list header operations, causing out-of-bounds
    /// access or other structural violations).
    pub unsafe fn iter_from(&self, start: SectorIndex) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: start,
            sectors: self.sectors,
        }
    }
}

pub struct LinkedListIter<'a> {
    pub curr: SectorIndex,
    pub sectors: &'a [u8],
}

impl<'a> Iterator for LinkedListIter<'a> {
    type Item = (SectorIndex, &'a Sector);

    /// Returns the next sector if it's non-NIL, otherwise, returns `None`.
    fn next(&mut self) -> Option<(SectorIndex, &'a Sector)> {
        if self.curr == NIL {
            return None;
        }

        // Safety: `self.curr` is non-NIL and per the linked list impl, must be in-bounds.
        let sector = unsafe { Sector::from_sector_index(self.sectors, self.curr) };
        let res = (self.curr, sector);

        self.curr = sector.next();
        Some(res)
    }
}
