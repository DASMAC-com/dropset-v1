use dropset_interface::{
    error::DropsetError,
    program,
    state::{
        market::{Market, MarketRef, MarketRefMut},
        market_header::MARKET_HEADER_SIZE,
        sector::SECTOR_SIZE,
    },
    utils::owned_by,
};
use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::shared::account_resize::fund_then_resize_unchecked;

#[derive(Clone)]
pub struct MarketAccountInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> MarketAccountInfo<'a> {
    #[inline(always)]
    /// Only checks that the account is owned by this program to allow the caller to validate
    /// internal program data on a case by case basis.
    pub fn new(info: &'a AccountInfo) -> Result<MarketAccountInfo<'a>, DropsetError> {
        if !owned_by(info, &program::ID) {
            return Err(DropsetError::InvalidMarketAccountOwner);
        }

        Ok(Self { info })
    }

    #[inline(always)]
    /// Helper function to load market data given the owner-validated market account.
    ///
    /// NOTE: Market account data may be uninitialized and isn't verified here.
    ///
    /// # Safety
    /// Caller guarantees:
    /// - There are no active mutable borrows of the market account data.
    /// - There are <= 6 active borrows of the market account data.
    pub unsafe fn load_unchecked(&self) -> Result<MarketRef, DropsetError> {
        let data = unsafe { self.info.borrow_data_unchecked() };
        Market::from_bytes_unchecked(data)
    }

    #[inline(always)]
    /// Helper function to load market data given the owner-validated market account.
    ///
    /// NOTE: Market account data may be uninitialized and isn't verified here.
    ///
    /// # Safety
    /// Caller guarantees:
    /// - There are no active borrows of the market account data.
    pub unsafe fn load_unchecked_mut(&self) -> Result<MarketRefMut, DropsetError> {
        let data = unsafe { self.info.borrow_mut_data_unchecked() };
        Market::from_bytes_mut_unchecked(data)
    }

    #[inline(always)]
    /// Resizes the market account data and then initializes free nodes onto the free stack by
    /// calculating the available space as a factor of SECTOR_SIZE.
    ///
    /// # Safety
    ///
    /// Caller must guarantee that there are no active borrows of the market account's data.
    pub unsafe fn resize(&self, payer: &AccountInfo, num_sectors: u16) -> ProgramResult {
        if num_sectors == 0 {
            return Err(DropsetError::InvalidNonZeroInteger.into());
        }

        let curr_n_sectors = (self.info.data_len() - MARKET_HEADER_SIZE) / SECTOR_SIZE;
        let new_n_sectors = curr_n_sectors + (num_sectors as usize);
        let additional_space = (num_sectors as usize) * SECTOR_SIZE;

        // Safety: Caller must guarantee no active borrows on the market's account data.
        let data = unsafe {
            fund_then_resize_unchecked(payer, self.info, additional_space)?;
            self.info.borrow_mut_data_unchecked()
        };

        let mut market = Market::from_bytes_mut_unchecked(data)?;
        let mut stack = market.free_stack();

        // Safety: Account data just zero-initialized new account space, and both indices are in
        // bounds and non-NIL.
        unsafe { stack.push_free_nodes(curr_n_sectors as u32, new_n_sectors as u32) }?;

        Ok(())
    }
}
