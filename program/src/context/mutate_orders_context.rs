//! See [`MutateOrdersContext`].

use dropset_interface::instructions::generated_program::PostOrder;
use pinocchio::{
    account::AccountView,
    error::ProgramError,
};

use crate::validation::market_account_view::MarketAccountView;

/// The account context for any instruction that mutates a user's orders (e.g. post or cancel),
/// validating the market account passed in.
#[derive(Clone)]
pub struct MutateOrdersContext<'a> {
    // The event authority is validated by the inevitable `FlushEvents` self-CPI.
    pub event_authority: &'a AccountView,
    pub user: &'a AccountView,
    pub market_account: MarketAccountView<'a>,
}

impl<'a> MutateOrdersContext<'a> {
    /// # Safety
    ///
    /// Caller guarantees no accounts passed have their data borrowed in any capacity. This is a
    /// more restrictive safety contract than is necessary for soundness but is much simpler.
    pub unsafe fn load(
        accounts: &'a [AccountView],
    ) -> Result<MutateOrdersContext<'a>, ProgramError> {
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

#[cfg(test)]
pub(crate) mod tests {
    use dropset_interface::instructions::generated_program::{
        BatchReplace,
        CancelOrder,
        PostOrder,
    };
    use pinocchio::{
        AccountView,
        Address,
    };
    use solana_account_view::RuntimeAccount;

    use crate::context::deposit_withdraw_context::tests::{
        assert_address_eq,
        create_zeroed_mock_runtime_account,
    };

    #[test]
    fn mutate_orders_account_order_invariant() {
        let mut runtime_accounts = [
            create_zeroed_mock_runtime_account(Address::new_from_array([0u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([1u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([2u8; 32])),
            create_zeroed_mock_runtime_account(Address::new_from_array([3u8; 32])),
        ];

        let accounts_ptr: *mut RuntimeAccount = runtime_accounts.as_mut_ptr();

        let account_views = unsafe {
            [
                AccountView::new_unchecked(accounts_ptr.add(0)),
                AccountView::new_unchecked(accounts_ptr.add(1)),
                AccountView::new_unchecked(accounts_ptr.add(2)),
                AccountView::new_unchecked(accounts_ptr.add(3)),
            ]
        };

        let po = PostOrder::load_accounts(&account_views).unwrap();
        let ca = CancelOrder::load_accounts(&account_views).unwrap();
        let br = BatchReplace::load_accounts(&account_views).unwrap();

        // Destructure to ensure that all account info fields are accounted for.
        // This isn't necessary for the other two structs because `load_accounts` would fail if they
        // receive fewer accounts than necessary, and the comparisons below ensure they have
        // at least as many fields.
        let PostOrder {
            event_authority,
            user,
            market_account,
            dropset_program,
        } = po;

        // Ensure the accounts are loaded in the same exact order by comparing each unique address.
        assert_address_eq(ca.event_authority, event_authority);
        assert_address_eq(ca.user, user);
        assert_address_eq(ca.market_account, market_account);
        assert_address_eq(ca.dropset_program, dropset_program);

        assert_address_eq(br.event_authority, event_authority);
        assert_address_eq(br.user, user);
        assert_address_eq(br.market_account, market_account);
        assert_address_eq(br.dropset_program, dropset_program);
    }
}
