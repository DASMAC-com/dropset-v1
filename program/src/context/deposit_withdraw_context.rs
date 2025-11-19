//! See [`DepositWithdrawContext`].

use dropset_interface::instructions::generated_pinocchio::Deposit;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::validation::{
    market_account_info::MarketAccountInfo,
    mint_info::MintInfo,
    token_account_info::TokenAccountInfo,
};

/// The account context for the [`Deposit`] and
/// [`dropset_interface::instructions::generated_pinocchio::Withdraw`] instructions, verifying token
/// ownership, mint consistency, and associated token account correctness.
#[derive(Clone)]
pub struct DepositWithdrawContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountInfo,
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
        // Ensure no drift between deposit/withdraw struct fields since this method is used to load
        // accounts for both `Deposit` and `Withdraw` instructions.
        // Ideally, this would be a unit test, but it's not possible to construct the `pinocchio`
        // `AccountInfo` without spinning up an entire e2e test with a local validator.
        #[cfg(debug_assertions)]
        debug_assert_deposit_withdraw(accounts);

        // Note `Withdraw`'s fields are checked below with unit tests, since this method is used for
        // both `Deposit` and `Withdraw`.
        let Deposit {
            event_authority,
            user,
            market_account,
            user_ata,
            market_ata,
            mint,
            token_program: _,
            dropset_program: _,
        } = Deposit::load_accounts(accounts)?;

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
            event_authority,
            user,
            market_account,
            user_ata,
            market_ata,
            mint,
        })
    }
}

#[cfg(debug_assertions)]
fn debug_assert_deposit_withdraw(accounts: &[AccountInfo]) {
    use dropset_interface::instructions::generated_pinocchio::{
        Deposit,
        Withdraw,
    };

    let d = Deposit::load_accounts(accounts);
    let w = Withdraw::load_accounts(accounts);

    debug_assert_eq!(d.is_ok(), w.is_ok(), "Deposit/Withdraw mapping drift");

    // Let the `load` function handle the error.
    if d.is_err() {
        return;
    }

    // The compiler will raise an error if these fields are incorrect.
    let Withdraw {
        event_authority,
        user,
        market_account,
        user_ata,
        market_ata,
        mint,
        token_program,
        dropset_program,
    } = w.unwrap();

    let d = d.unwrap();

    // And to ensure the same ordering, check the pubkeys field by field.
    debug_assert_eq!(d.event_authority.key(), event_authority.key());
    debug_assert_eq!(d.user.key(), user.key());
    debug_assert_eq!(d.market_account.key(), market_account.key());
    debug_assert_eq!(d.user_ata.key(), user_ata.key());
    debug_assert_eq!(d.market_ata.key(), market_ata.key());
    debug_assert_eq!(d.mint.key(), mint.key());
    debug_assert_eq!(d.token_program.key(), token_program.key());
    debug_assert_eq!(d.dropset_program.key(), dropset_program.key());
}
