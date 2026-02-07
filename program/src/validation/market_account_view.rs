//! See [`MarketAccountView`].

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
        sector::{
            Sector,
            SECTOR_SIZE,
        },
        transmutable::Transmutable,
    },
    utils::owned_by,
};
use pinocchio::{
    account::AccountView,
    hint::unlikely,
    ProgramResult,
};

use crate::shared::account_resize::fund_then_resize_unchecked;

/// A validated wrapper around a raw market [`AccountView`], providing safe access
/// to the market header and sector data after verifying ownership and layout.
#[derive(Clone)]
pub struct MarketAccountView<'a> {
    /// The account view as a private field. This disallows manual construction, guaranteeing an
    /// extra level of safety and simplifying the safety contracts for the unsafe internal methods.
    account: &'a AccountView,
}

impl<'a> MarketAccountView<'a> {
    #[inline(always)]
    pub fn account(&self) -> &'a AccountView {
        self.account
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
    pub unsafe fn new(account: &'a AccountView) -> Result<MarketAccountView<'a>, DropsetError> {
        if unlikely(!owned_by(account, &program::ID)) {
            return Err(DropsetError::InvalidMarketAccountOwner);
        }

        let data = unsafe { account.borrow_unchecked() };
        if unlikely(data.len() < MarketHeader::LEN) {
            return Err(DropsetError::AccountNotInitialized);
        }

        // Sector size alignment is an invariant enforced by market initialization and resize
        // functions. This check is here as a sanity check.
        debug_assert_eq!((data.len() - MarketHeader::LEN) % Sector::LEN, 0);

        // Safety: The owner and initialization state was just verified.
        let market = unsafe { Market::from_bytes(data) };
        if unlikely(!(market.is_initialized())) {
            return Err(DropsetError::AccountNotInitialized);
        }

        Ok(Self { account })
    }

    /// Safety:
    ///
    /// Caller guarantees that `account` is a valid, initialized market account.
    pub unsafe fn new_unchecked(account: &'a AccountView) -> MarketAccountView<'a> {
        Self { account }
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
        let data = unsafe { self.account.borrow_unchecked() };
        // Safety: Assumes the `Self` invariant: the market account is program-owned & initialized.
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
        let data = unsafe { self.account.borrow_unchecked_mut() };
        // Safety: Assumes the `Self` invariant: the market account is program-owned & initialized.
        unsafe { Market::from_bytes_mut(data) }
    }

    /// Resizes the market account data and then initializes free sectors onto the free stack by
    /// calculating the available space as a factor of [`Sector::LEN`].
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
    pub unsafe fn resize(&mut self, payer: &AccountView, num_sectors: u16) -> ProgramResult {
        if unlikely(num_sectors == 0) {
            return Err(DropsetError::InvalidNonZeroInteger.into());
        }

        // Safety: No underflow possible here if the market header has been initialized, which is
        // part of the safety contract for creating `Self`.
        let curr_n_sectors =
            unsafe { (self.account.data_len().unchecked_sub(MarketHeader::LEN)) / SECTOR_SIZE };
        let new_n_sectors = curr_n_sectors + (num_sectors as usize);
        let additional_space = (num_sectors as usize) * SECTOR_SIZE;

        // Safety: Scoped writes to payer and market account to resize the market account.
        unsafe { fund_then_resize_unchecked(payer, self.account, additional_space) }?;

        // Safety: Mutably borrows market account data for the rest of this function.
        let mut market = unsafe { self.load_unchecked_mut() };
        let mut stack = market.free_stack();

        // Safety: Account data just zero-initialized new account space, and both indices are in
        // bounds and non-NIL.
        unsafe {
            stack.convert_zeroed_bytes_to_free_sectors(curr_n_sectors as u32, new_n_sectors as u32)
        }?;

        Ok(())
    }
}
