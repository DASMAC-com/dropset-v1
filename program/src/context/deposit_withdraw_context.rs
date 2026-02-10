//! See [`DepositWithdrawContext`].

use dropset_interface::instructions::generated_program::Deposit;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::{
    market_account_view::MarketAccountView,
    mint_account_view::MintAccountView,
    token_account_view::TokenAccountView,
};

/// The account context for the [`Deposit`] and
/// [`dropset_interface::instructions::generated_program::Withdraw`] instructions, verifying token
/// ownership, mint consistency, and associated token account correctness.
#[derive(Clone)]
pub struct DepositWithdrawContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountView,
    pub user: &'a AccountView,
    pub market_account: MarketAccountView<'a>,
    pub user_ata: TokenAccountView<'a>,
    pub market_ata: TokenAccountView<'a>,
    pub mint: MintAccountView<'a>,
}

impl<'a> DepositWithdrawContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees no accounts passed have their data borrowed in any capacity. This is a
    /// more restrictive safety contract than is necessary for soundness but is much simpler.
    pub unsafe fn load(
        accounts: &'a [AccountView],
    ) -> Result<DepositWithdrawContext<'a>, ProgramError> {
        // `Withdraw`'s account info fields are in the same exact order as `Deposit`'s, so just use
        // `Deposit::load_accounts` for both. This invariant is checked below in unit tests.
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
            let market_account = MarketAccountView::new(market_account)?;
            let market = market_account.load_unchecked();
            let mint = MintAccountView::new(mint, market)?;
            (market_account, mint)
        };

        // Safety: Scoped borrows of the user token account and market token account.
        let (user_ata, market_ata) = unsafe {
            let user_ata = TokenAccountView::new(user_ata, mint.account.address(), user.address())?;
            let market_ata = TokenAccountView::new(
                market_ata,
                mint.account.address(),
                market_account.account().address(),
            )?;
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

#[cfg(test)]
pub(crate) mod tests {
    use dropset_interface::{
        instructions::generated_program::{
            Deposit,
            Withdraw,
        },
        state::SYSTEM_PROGRAM_ID,
    };
    use pinocchio::{
        AccountView,
        Address,
    };
    use solana_account_view::RuntimeAccount;

    /// Creates a mock runtime account with only the address field not set to zeros.
    pub(crate) fn create_zeroed_mock_runtime_account(address: Address) -> RuntimeAccount {
        RuntimeAccount {
            borrow_state: 0,
            is_signer: 0,
            is_writable: 0,
            executable: 0,
            resize_delta: 0,
            // Address is the only field that matters, because these tests are solely checking for
            // matching named account info field ordering in multiple instruction contexts.
            address,
            owner: SYSTEM_PROGRAM_ID,
            lamports: 0,
            data_len: 0,
        }
    }

    pub(crate) fn assert_address_eq(view_1: &AccountView, view_2: &AccountView) {
        assert_eq!(view_1.address(), view_2.address());
    }

    #[test]
    fn deposit_withdraw_account_order_invariant() {
        let mut runtime_accounts = [
            create_zeroed_mock_runtime_account(Address::new_from_array([0u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([1u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([2u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([3u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([4u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([5u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([6u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([7u8; 32])),
        ];

        let accounts_ptr: *mut RuntimeAccount = runtime_accounts.as_mut_ptr();

        let account_views = unsafe {
            [
                AccountView::new_unchecked(accounts_ptr.add(0)),
                AccountView::new_unchecked(accounts_ptr.add(1)),
                AccountView::new_unchecked(accounts_ptr.add(2)),
                AccountView::new_unchecked(accounts_ptr.add(3)),
                AccountView::new_unchecked(accounts_ptr.add(4)),
                AccountView::new_unchecked(accounts_ptr.add(5)),
                AccountView::new_unchecked(accounts_ptr.add(6)),
                AccountView::new_unchecked(accounts_ptr.add(7)),
            ]
        };

        let dep = Deposit::load_accounts(&account_views).unwrap();
        let wd = Withdraw::load_accounts(&account_views).unwrap();

        // Destructure to ensure that all account info fields are accounted for.
        // This isn't necessary for `Deposit` because `load_accounts` would fail if it has fewer
        // accounts than necessary, and the comparisons below ensure it has at least as many fields.
        let Withdraw {
            event_authority,
            user,
            market_account,
            user_ata,
            market_ata,
            mint,
            token_program,
            dropset_program,
        } = wd;

        // Ensure the accounts are loaded in the same exact order by comparing each unique address.
        assert_address_eq(dep.event_authority, event_authority);
        assert_address_eq(dep.user, user);
        assert_address_eq(dep.market_account, market_account);
        assert_address_eq(dep.user_ata, user_ata);
        assert_address_eq(dep.market_ata, market_ata);
        assert_address_eq(dep.mint, mint);
        assert_address_eq(dep.token_program, token_program);
        assert_address_eq(dep.dropset_program, dropset_program);
    }
}
