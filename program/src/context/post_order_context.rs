//! See [`PostOrderContext`].

use dropset_interface::instructions::generated_program::PostOrder;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::market_account_view::MarketAccountView;

/// The account context for the [`PostOrder`] instruction, validating the market account passed in.
#[derive(Clone)]
pub struct PostOrderContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountView,
    pub user: &'a AccountView,
    pub market_account: MarketAccountView<'a>,
}

impl<'a> PostOrderContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Market account
    pub unsafe fn load(accounts: &'a [AccountView]) -> Result<PostOrderContext<'a>, ProgramError> {
        let PostOrder {
            event_authority,
            user,
            market_account,
            dropset_program: _,
        } = PostOrder::load_accounts(accounts)?;

        // Safety: Scoped borrow of market account data.
        let market_account = unsafe { MarketAccountView::new(market_account) }?;

        Ok(Self {
            event_authority,
            user,
            market_account,
        })
    }
}
