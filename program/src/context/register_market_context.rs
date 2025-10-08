use dropset_interface::error::DropsetError;
use pinocchio::account_info::AccountInfo;

use crate::validation::{
    token_program_info::TokenProgramInfo, uninitialized_account_info::UninitializedAccountInfo,
};

#[derive(Clone)]
pub struct RegisterMarketContext<'a> {
    pub user: &'a AccountInfo,
    pub market_account: UninitializedAccountInfo<'a>,
    pub base_mint: &'a AccountInfo,
    pub quote_mint: &'a AccountInfo,
    pub base_market_ata: &'a AccountInfo,
    pub quote_market_ata: &'a AccountInfo,
    pub base_token_program: TokenProgramInfo<'a>,
    pub quote_token_program: TokenProgramInfo<'a>,
    pub system_program: &'a AccountInfo,
}

impl<'a> RegisterMarketContext<'a> {
    pub fn load(accounts: &'a [AccountInfo]) -> Result<RegisterMarketContext<'a>, DropsetError> {
        let [user, market_account, base_mint, quote_mint, base_market_ata, quote_market_ata, base_token_program, quote_token_program, system_program] =
            accounts
        else {
            return Err(DropsetError::NotEnoughAccountKeys);
        };

        // Since the market PDA and both of its associated token accounts are created atomically
        // during market registration, all derivations are guaranteed to be correct if the
        // transaction succeeds. The two mint accounts are also guaranteed to be different, since
        // the non-idempotent ATA creation instruction would fail on the second invocation.
        // Thus there is no need to check ownership, address derivations, or account data here, only
        // that the token programs provided are valid and that `market_account` is uninitialized.
        let market_account = UninitializedAccountInfo::new(market_account)?;
        // These checks are necessary since an antagonistic caller could substitute these with
        // malicious programs with identical interfaces as the token programs.
        let base_token_program = TokenProgramInfo::new(base_token_program)?;
        let quote_token_program = TokenProgramInfo::new(quote_token_program)?;

        Ok(Self {
            user,
            market_account,
            base_mint,
            quote_mint,
            base_market_ata,
            quote_market_ata,
            base_token_program,
            quote_token_program,
            system_program,
        })
    }
}
