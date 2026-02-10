//! See [`MarketOrderContext`].

use dropset_interface::instructions::generated_program::MarketOrder;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::{
    market_account_view::MarketAccountView,
    mint_account_view::MintAccountView,
    token_account_view::TokenAccountView,
};

/// The contextual, validated account infos required for a market order.
#[derive(Clone)]
pub struct MarketOrderContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountView,
    pub user: &'a AccountView,
    pub market_account: MarketAccountView<'a>,
    pub base_user_ata: TokenAccountView<'a>,
    pub quote_user_ata: TokenAccountView<'a>,
    pub base_market_ata: TokenAccountView<'a>,
    pub quote_market_ata: TokenAccountView<'a>,
    pub base_mint: MintAccountView<'a>,
    pub quote_mint: MintAccountView<'a>,
}

impl<'a> MarketOrderContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees no accounts passed have their data borrowed in any capacity. This is a
    /// more restrictive safety contract than is necessary for soundness but is much simpler.
    pub unsafe fn load(
        accounts: &'a [AccountView],
    ) -> Result<MarketOrderContext<'a>, ProgramError> {
        let MarketOrder {
            event_authority,
            user,
            market_account,
            base_user_ata,
            quote_user_ata,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
            base_token_program: _,
            quote_token_program: _,
            dropset_program: _,
        } = MarketOrder::load_accounts(accounts)?;

        // Safety: Scoped borrow of market account data.
        let (market_account, base_mint, quote_mint) = unsafe {
            let market_account = MarketAccountView::new(market_account)?;
            let market = market_account.load_unchecked();
            let (base_mint, quote_mint) =
                MintAccountView::new_base_and_quote(base_mint, quote_mint, market)?;
            (market_account, base_mint, quote_mint)
        };

        // Safety: Scoped borrows of the user token account and market token account.
        let (base_user_ata, base_market_ata, quote_user_ata, quote_market_ata) = unsafe {
            let base_user_ata =
                TokenAccountView::new(base_user_ata, base_mint.account.address(), user.address())?;
            let base_market_ata = TokenAccountView::new(
                base_market_ata,
                base_mint.account.address(),
                market_account.account().address(),
            )?;
            let quote_user_ata = TokenAccountView::new(
                quote_user_ata,
                quote_mint.account.address(),
                user.address(),
            )?;
            let quote_market_ata = TokenAccountView::new(
                quote_market_ata,
                quote_mint.account.address(),
                market_account.account().address(),
            )?;
            (
                base_user_ata,
                base_market_ata,
                quote_user_ata,
                quote_market_ata,
            )
        };

        Ok(Self {
            event_authority,
            user,
            market_account,
            base_user_ata,
            quote_user_ata,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
        })
    }
}
