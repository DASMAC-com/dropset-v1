use crate::state::{
    free_stack::Stack,
    linked_list::{LinkedList, LinkedListIter},
    market_header::{MarketHeader, MARKET_ACCOUNT_DISCRIMINANT},
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
    fn as_ref(&self) -> &MarketHeader {
        self
    }
}

impl AsMut<MarketHeader> for &mut MarketHeader {
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
    /// Caller guarantees that `MARKET_HEADER_SIZE <= data.len()`.
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
    /// Caller guarantees that `MARKET_HEADER_SIZE <= data.len()`.
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
    pub fn seat_list(&mut self) -> LinkedList<'_> {
        LinkedList::new_from_parts(self.header, self.sectors)
    }
}

impl<H: AsRef<MarketHeader>, S: AsRef<[u8]>> Market<H, S> {
    #[inline(always)]
    pub fn iter_seats(&self) -> LinkedListIter {
        LinkedListIter {
            curr: self.header.as_ref().seat_dll_head(),
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
