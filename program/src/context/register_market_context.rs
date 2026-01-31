//! See [`RegisterMarketContext`].

use dropset_interface::instructions::generated_program::RegisterMarket;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::uninitialized_account_view::UninitializedAccountView;

/// The account context for the [`RegisterMarket`] instruction, validating ownership,
/// initialization, and PDA derivations for market creation.
#[derive(Clone)]
pub struct RegisterMarketContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountView,
    pub user: &'a AccountView,
    pub market_account: UninitializedAccountView<'a>,
    pub base_market_ata: &'a AccountView,
    pub quote_market_ata: &'a AccountView,
    pub base_mint: &'a AccountView,
    pub quote_mint: &'a AccountView,
    pub base_token_program: &'a AccountView,
    pub quote_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> RegisterMarketContext<'a> {
    pub fn load(accounts: &'a [AccountView]) -> Result<RegisterMarketContext<'a>, ProgramError> {
        let RegisterMarket {
            event_authority,
            user,
            market_account,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            ata_program: _,
            system_program,
            dropset_program: _,
        } = RegisterMarket::load_accounts(accounts)?;

        // Since the market PDA and both of its associated token accounts are created atomically
        // during market registration, all derivations are guaranteed to be correct if the
        // transaction succeeds. The two mint accounts are also guaranteed to be different, since
        // the non-idempotent ATA creation instruction would fail on the second invocation.
        // Thus there is no need to check ownership, address derivations, or account data here, only
        // that the `market_account` is uninitialized.
        // The token programs are also validated in the ATA `Create` instruction.
        let market_account = UninitializedAccountView::new(market_account)?;

        Ok(Self {
            event_authority,
            user,
            market_account,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            system_program,
        })
    }
}
