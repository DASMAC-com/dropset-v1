use crate::{
    error::DropsetError,
    state::{
        free_stack::Stack,
        linked_list::LinkedList,
        market_header::{MarketHeader, MARKET_HEADER_SIZE},
        sector::SECTOR_SIZE,
        transmutable::{load_unchecked, load_unchecked_mut},
    },
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
    /// Checks:
    /// 1. The data passed in is long enough to represent a Market.
    /// 2. The discriminant in the header matches the expected one written during initialization.
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety: `split_at_*` ensures `header_bytes == MarketHeader::LEN`, and MarketHeaders are
        // valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked::<MarketHeader>(header_bytes) };
        header.verify_discriminant()?;

        Ok(Self { header, sectors })
    }
}

impl<'a> MarketRefMut<'a> {
    /// Returns mutable references to a Market's header and sectors slice.
    ///
    /// Checks:
    /// 1. The data passed in is long enough to represent a Market.
    /// 2. The discriminant in the header matches the expected one written during initialization.
    pub fn from_bytes_mut(data: &'a mut [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_mut_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety:
        // - `split_at_*` ensures `header_bytes == MarketHeader::LEN`.
        // - MarketHeaders are valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked_mut::<MarketHeader>(header_bytes) };
        header.verify_discriminant()?;

        Ok(Self { header, sectors })
    }

    /// Returns mutable references to a Market's header and sectors slice without checking the data.
    ///
    /// This function should only be called if `data` is well-formed, initialized market data.
    pub fn from_bytes_mut_unchecked(data: &'a mut [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_mut_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety:
        // - `split_at_*` ensures `header_bytes == MarketHeader::LEN`.
        // - MarketHeaders are valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked_mut::<MarketHeader>(header_bytes) };

        Ok(Self { header, sectors })
    }

    #[inline(always)]
    pub fn free_stack(&mut self) -> Stack<'_> {
        Stack::new_from_parts(self.header.as_mut().free_stack_top_mut_ref(), self.sectors)
    }

    #[inline(always)]
    pub fn seat_list(&mut self) -> LinkedList<'_> {
        LinkedList::new_from_parts(self.header, self.sectors)
    }
}
