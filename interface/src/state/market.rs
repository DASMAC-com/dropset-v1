//! The top-level market structure tying together header, seats, and
//! storage sectors into a unified on-chain representation.

use crate::state::{
    asks_dll::AskOrdersLinkedList,
    bids_dll::BidOrdersLinkedList,
    free_stack::Stack,
    linked_list::{
        LinkedList,
        LinkedListIter,
    },
    market_header::{
        MarketHeader,
        MARKET_ACCOUNT_DISCRIMINANT,
    },
    order::OrdersCollection,
    seats_dll::SeatsLinkedList,
    sector::SECTOR_SIZE,
    transmutable::Transmutable,
};

pub struct Market<Header, SectorBytes> {
    pub header: Header,
    pub sectors: SectorBytes,
}

pub type MarketRef<'a> = Market<&'a MarketHeader, &'a [u8]>;
pub type MarketRefMut<'a> = Market<&'a mut MarketHeader, &'a mut [u8]>;

impl AsRef<MarketHeader> for &MarketHeader {
    #[inline(always)]
    fn as_ref(&self) -> &MarketHeader {
        self
    }
}

impl AsRef<MarketHeader> for &mut MarketHeader {
    #[inline(always)]
    fn as_ref(&self) -> &MarketHeader {
        self
    }
}

impl AsMut<MarketHeader> for &mut MarketHeader {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut MarketHeader {
        self
    }
}

impl<'a> MarketRef<'a> {
    /// Returns immutable references to a Market's header and sectors slice.
    ///
    /// Checking that `data` is owned by a Market account and that the slices have initialized data
    /// is left up to the caller.
    ///
    /// # Safety
    ///
    /// Caller guarantees that `data.len() >= MARKET_HEADER_SIZE`.
    pub unsafe fn from_bytes(data: &'a [u8]) -> Self {
        let (header_bytes, sectors) = data.split_at_unchecked(MarketHeader::LEN);
        // Safety: MarketHeaders are valid for all bit patterns.
        let header = unsafe { MarketHeader::load_unchecked(header_bytes) };

        Self { header, sectors }
    }
}

impl<'a> MarketRefMut<'a> {
    /// Returns mutable references to a Market's header and sectors slice.
    ///
    /// Checking that `data` is owned by a Market account and that the slices have initialized data
    /// is left up to the caller.
    ///
    /// # Safety
    ///
    /// Caller guarantees that `data.len() >= MARKET_HEADER_SIZE`.
    pub unsafe fn from_bytes_mut(data: &'a mut [u8]) -> Self {
        let (header_bytes, sectors) = data.split_at_mut_unchecked(MarketHeader::LEN);
        // Safety: MarketHeaders are valid (no undefined behavior) for all bit patterns.
        let header = unsafe { MarketHeader::load_unchecked_mut(header_bytes) };

        Self { header, sectors }
    }

    #[inline(always)]
    pub fn free_stack(&mut self) -> Stack<'_> {
        Stack::new_from_parts(self.header, self.sectors)
    }

    #[inline(always)]
    pub fn seats(&mut self) -> SeatsLinkedList {
        SeatsLinkedList::new_from_parts(self.header, self.sectors)
    }

    #[inline(always)]
    pub fn bids(&mut self) -> BidOrdersLinkedList {
        BidOrdersLinkedList::new_from_parts(self.header, self.sectors)
    }

    #[inline(always)]
    pub fn asks(&mut self) -> AskOrdersLinkedList {
        AskOrdersLinkedList::new_from_parts(self.header, self.sectors)
    }

    #[inline(always)]
    pub fn orders<T: OrdersCollection>(
        &mut self,
    ) -> LinkedList<'_, T> {
        LinkedList::new_from_parts(self.header, self.sectors)
    }
}

impl<H: AsRef<MarketHeader>, S: AsRef<[u8]>> Market<H, S> {
    #[inline(always)]
    pub fn iter_bids(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: self.header.as_ref().bids_dll_head(),
            sectors: self.sectors.as_ref(),
        }
    }

    #[inline(always)]
    pub fn iter_asks(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: self.header.as_ref().asks_dll_head(),
            sectors: self.sectors.as_ref(),
        }
    }

    #[inline(always)]
    pub fn iter_seats(&self) -> LinkedListIter<'_> {
        LinkedListIter {
            curr: self.header.as_ref().seats_dll_head(),
            sectors: self.sectors.as_ref(),
        }
    }

    #[inline(always)]
    pub fn get_capacity(&self) -> u32 {
        (self.sectors.as_ref().len() / SECTOR_SIZE) as u32
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.header.as_ref().discriminant() == MARKET_ACCOUNT_DISCRIMINANT
    }
}
