//! See [`MarketOrderContext`].

use dropset_interface::instructions::generated_pinocchio::MarketOrder;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::validation::{
    market_account_info::MarketAccountInfo,
    mint_info::MintInfo,
    token_account_info::TokenAccountInfo,
};

/// The contextual, validated account infos required for a market order.
#[derive(Clone)]
pub struct MarketOrderContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountInfo,
    pub user: &'a AccountInfo,
    pub market_account: MarketAccountInfo<'a>,
    pub base_user_ata: TokenAccountInfo<'a>,
    pub quote_user_ata: TokenAccountInfo<'a>,
    pub base_market_ata: TokenAccountInfo<'a>,
    pub quote_market_ata: TokenAccountInfo<'a>,
    pub base_mint: MintInfo<'a>,
    pub quote_mint: MintInfo<'a>,
}

impl<'a> MarketOrderContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Market account
    ///   1. `[READ]` Base user token account
    ///   2. `[READ]` Quote user token account
    ///   3. `[READ]` Base market token account
    ///   4. `[READ]` Quote market token account
    pub unsafe fn load(
        accounts: &'a [AccountInfo],
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
            let market_account = MarketAccountInfo::new(market_account)?;
            let market = market_account.load_unchecked();
            let (base_mint, quote_mint) =
                MintInfo::new_base_and_quote(base_mint, quote_mint, market)?;
            (market_account, base_mint, quote_mint)
        };

        // Safety: Scoped borrows of the user token account and market token account.
        let (base_user_ata, base_market_ata, quote_user_ata, quote_market_ata) = unsafe {
            let base_user_ata =
                TokenAccountInfo::new(base_user_ata, base_mint.info.key(), user.key())?;
            let base_market_ata = TokenAccountInfo::new(
                base_market_ata,
                base_mint.info.key(),
                market_account.info().key(),
            )?;
            let quote_user_ata =
                TokenAccountInfo::new(quote_user_ata, quote_mint.info.key(), user.key())?;
            let quote_market_ata = TokenAccountInfo::new(
                quote_market_ata,
                quote_mint.info.key(),
                market_account.info().key(),
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
