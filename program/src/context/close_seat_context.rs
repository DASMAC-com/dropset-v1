//! See [`CloseSeatContext`].

use dropset_interface::instructions::generated_pinocchio::CloseSeat;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::validation::{
    market_account_info::MarketAccountInfo,
    mint_info::MintInfo,
    token_account_info::TokenAccountInfo,
};

/// The account context for the [`CloseSeat`] instruction, ensuring the seat and related resources
/// are valid for closure.
#[derive(Clone)]
pub struct CloseSeatContext<'a> {
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

impl<'a> CloseSeatContext<'a> {
    pub unsafe fn load(accounts: &'a [AccountInfo]) -> Result<CloseSeatContext<'a>, ProgramError> {
        let CloseSeat {
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
        } = CloseSeat::load_accounts(accounts)?;

        // Safety: Scoped borrow of market account data.
        let (market_account, base_mint, quote_mint) = unsafe {
            let market_account = MarketAccountInfo::new(market_account)?;
            let market = market_account.load_unchecked();
            // Check the base and quote mints against the mints in the market header.
            let (base_mint, quote_mint) =
                MintInfo::new_base_and_quote(base_mint, quote_mint, market)?;
            (market_account, base_mint, quote_mint)
        };

        // Safety: Scoped borrows of the various user/market + base/quote token accounts.
        let base_user_ata = TokenAccountInfo::new(base_user_ata, base_mint.info.key(), user.key())?;
        let quote_user_ata =
            TokenAccountInfo::new(quote_user_ata, quote_mint.info.key(), user.key())?;
        let base_market_ata = TokenAccountInfo::new(
            base_market_ata,
            base_mint.info.key(),
            market_account.info().key(),
        )?;
        let quote_market_ata = TokenAccountInfo::new(
            quote_market_ata,
            quote_mint.info.key(),
            market_account.info().key(),
        )?;

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
