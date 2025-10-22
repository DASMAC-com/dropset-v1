use dropset_interface::error::DropsetError;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::validation::{
    market_account_info::MarketAccountInfo,
    mint_info::MintInfo,
    token_account_info::TokenAccountInfo,
};

#[derive(Clone)]
pub struct DepositWithdrawContext<'a> {
    pub user: &'a AccountInfo,
    pub market_account: MarketAccountInfo<'a>,
    pub user_ata: TokenAccountInfo<'a>,
    pub market_ata: TokenAccountInfo<'a>,
    pub mint: MintInfo<'a>,
}

impl<'a> DepositWithdrawContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Market account
    ///   1. `[READ]` User token account
    ///   2. `[READ]` Market token account
    pub unsafe fn load(
        accounts: &'a [AccountInfo],
    ) -> Result<DepositWithdrawContext<'a>, ProgramError> {
        #[rustfmt::skip]
        let [
            user,
            market_account,
            user_ata,
            market_ata,
            mint,
        ] = accounts else {
            return Err(DropsetError::NotEnoughAccountKeys.into());
        };

        // Safety: Scoped borrow of market account data.
        let (market_account, mint) = unsafe {
            let market_account = MarketAccountInfo::new(market_account)?;
            let market = market_account.load_unchecked();
            let mint = MintInfo::new(mint, market)?;
            (market_account, mint)
        };

        // Safety: Scoped borrows of the user token account and market token account.
        let (user_ata, market_ata) = unsafe {
            let user_ata = TokenAccountInfo::new(user_ata, mint.info.key(), user.key())?;
            let market_ata =
                TokenAccountInfo::new(market_ata, mint.info.key(), market_account.info().key())?;
            (user_ata, market_ata)
        };

        Ok(Self {
            user,
            market_account,
            user_ata,
            market_ata,
            mint,
        })
    }
}
