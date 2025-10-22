use dropset_interface::{
    error::DropsetError,
    program,
    state::{
        market::{
            Market,
            MarketRef,
            MarketRefMut,
        },
        market_header::MarketHeader,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    },
    utils::owned_by,
};
use pinocchio::{
    account_info::AccountInfo,
    ProgramResult,
};

use crate::shared::account_resize::fund_then_resize_unchecked;

#[derive(Clone)]
pub struct MarketAccountInfo<'a> {
    /// The account info as a private field. This disallows manual construction, guaranteeing an
    /// extra level of safety and simplifying the safety contracts for the unsafe internal methods.
    info: &'a AccountInfo,
}

impl<'a> MarketAccountInfo<'a> {
    #[inline(always)]
    pub fn info(&self) -> &'a AccountInfo {
        self.info
    }

    /// Checks that the account is owned by this program and is a properly initialized `Market`.
    ///
    /// ## NOTE
    ///
    /// The safety contract is only guaranteed if market accounts are never resized below the
    /// header size after initialization. If this invariant isn't always upheld, the validation
    /// performed by this method isn't guaranteed permanently.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Market account
    #[inline(always)]
    pub unsafe fn new(info: &'a AccountInfo) -> Result<MarketAccountInfo<'a>, DropsetError> {
        if !owned_by(info, &program::ID) {
            return Err(DropsetError::InvalidMarketAccountOwner);
        }

        let data = unsafe { info.borrow_data_unchecked() };
        if data.len() < MarketHeader::LEN {
            return Err(DropsetError::AccountNotInitialized);
        }

        if !(Market::from_bytes(data).is_initialized()) {
            return Err(DropsetError::AccountNotInitialized);
        }

        Ok(Self { info })
    }

    /// Helper function to load market data given the owner-validated and initialized account.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Market account
    #[inline(always)]
    pub unsafe fn load_unchecked(&self) -> MarketRef<'_> {
        let data = unsafe { self.info.borrow_data_unchecked() };
        // Safety: `Self::new` guarantees the account info is program-owned and initialized.
        unsafe { Market::from_bytes(data) }
    }

    /// Helper function to load market data given the owner-validated and initialized account.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[WRITE]` Market account
    #[inline(always)]
    pub unsafe fn load_unchecked_mut(&mut self) -> MarketRefMut<'_> {
        let data = unsafe { self.info.borrow_mut_data_unchecked() };
        // Safety: `Self::new` guarantees the account info is program-owned and initialized.
        unsafe { Market::from_bytes_mut(data) }
    }

    /// Resizes the market account data and then initializes free nodes onto the free stack by
    /// calculating the available space as a factor of SECTOR_SIZE.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[WRITE]` Payer
    ///   1. `[WRITE]` Market account
    #[inline(always)]
    pub unsafe fn resize(&mut self, payer: &AccountInfo, num_sectors: u16) -> ProgramResult {
        if num_sectors == 0 {
            return Err(DropsetError::InvalidNonZeroInteger.into());
        }

        let curr_n_sectors = (self.info.data_len() - MarketHeader::LEN) / SECTOR_SIZE;
        let new_n_sectors = curr_n_sectors + (num_sectors as usize);
        let additional_space = (num_sectors as usize) * SECTOR_SIZE;

        // Safety: Scoped writes to payer and market account to resize the market account.
        unsafe { fund_then_resize_unchecked(payer, self.info, additional_space) }?;

        // Safety: Mutably borrows market account data for the rest of this function.
        let mut market = unsafe { self.load_unchecked_mut() };
        let mut stack = market.free_stack();

        // Safety: Account data just zero-initialized new account space, and both indices are in
        // bounds and non-NIL.
        unsafe {
            stack.convert_zeroed_bytes_to_free_nodes(curr_n_sectors as u32, new_n_sectors as u32)
        }?;

        Ok(())
    }
}
