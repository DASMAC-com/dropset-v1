use pinocchio::pubkey::Pubkey;

use crate::{
    error::{DropsetError, DropsetResult},
    state::{
        free_stack::Stack,
        market_header::{MarketHeader, MARKET_HEADER_SIZE},
        sector::{SectorIndex, SECTOR_SIZE},
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
    pub fn from_bytes(data: &'a [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety: `split_at_*` ensures `header_bytes == MarketHeader::LEN`, and MarketHeaders are
        // valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked::<MarketHeader>(header_bytes) };
        Ok(Self { header, sectors })
    }
}

impl<'a> MarketRefMut<'a> {
    /// Verifies the account discriminant in the MarketHeader and that the sector bytes match
    /// the amount specified in the header.
    pub fn from_bytes_mut(data: &'a mut [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_mut_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety: `split_at_*` ensures `header_bytes == MarketHeader::LEN`, and MarketHeaders are
        // valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked_mut::<MarketHeader>(header_bytes) };

        if sectors.len() & SECTOR_SIZE != 0 {
            return Err(DropsetError::MismatchedDataLengths);
        }

        header.verify_discriminant()?;
        Ok(Self { header, sectors })
    }

    pub fn from_bytes_mut_unchecked(data: &'a mut [u8]) -> Result<Self, DropsetError> {
        let (header_bytes, sectors) = data
            .split_at_mut_checked(MARKET_HEADER_SIZE)
            .ok_or(DropsetError::InsufficientByteLength)?;

        // Safety: `split_at_*` ensures `header_bytes == MarketHeader::LEN`, and MarketHeaders are
        // valid (no undefined behavior) for all bit patterns.
        let header = unsafe { load_unchecked_mut::<MarketHeader>(header_bytes) };
        Ok(Self { header, sectors })
    }
}

impl<Header, SectorBytes> Market<Header, SectorBytes>
where
    Header: AsRef<MarketHeader>,
    SectorBytes: AsRef<[u8]>,
{
    pub fn init(
        // This data should only have been initialized with zeroes, nothing else.
        zeroed_market_account_data: &mut [u8],
        // TODO: Confirm this field can be properly removed, should be able to since the
        // remaining length after the header size is checked in the body of this function.
        // initial_num_sectors: u16,
        market_bump: u8,
        // TODO: Use verified reference: &'a MintInfo
        base_mint: &Pubkey,
        // TODO: Use verified reference: &'a MintInfo
        quote_mint: &Pubkey,
    ) -> DropsetResult {
        let account_data_len = zeroed_market_account_data.len();
        if account_data_len < MARKET_HEADER_SIZE {
            return Err(DropsetError::UnallocatedAccountData);
        }

        let sector_bytes = account_data_len - MARKET_HEADER_SIZE;

        if sector_bytes % SECTOR_SIZE != 0 {
            return Err(DropsetError::UnalignedData);
        }

        // Initialize the market header.
        let mut market = MarketRefMut::from_bytes_mut_unchecked(zeroed_market_account_data)?;
        *market.header = MarketHeader::init(market_bump, base_mint, quote_mint);

        // Initialize all sectors by adding them to the free stack.
        let stack = &mut market.free_stack();
        let num_sectors = sector_bytes / SECTOR_SIZE;
        for s in (0..num_sectors).rev() {
            stack.push_free_node(SectorIndex(s as u32))?;
        }

        Ok(())
    }
}

impl MarketRefMut<'_> {
    #[inline(always)]
    pub fn free_stack(&mut self) -> Stack<'_> {
        Stack::new_from_parts(self.header.as_mut().free_stack_top_mut_ref(), self.sectors)
    }
}
