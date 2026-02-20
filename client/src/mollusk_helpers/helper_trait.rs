use std::collections::HashMap;

use mollusk_svm::MolluskContext;
use solana_account::Account;
use solana_address::Address;
use solana_sdk::program_pack::Pack;
use spl_associated_token_account_interface::address::get_associated_token_address;
use spl_token_interface::state::Account as TokenAccount;
use transaction_parser::views::{
    try_market_view_all,
    MarketViewAll,
};

pub trait DropsetTestHelper {
    fn get_token_balance(&self, user: &Address, token_mint: &Address) -> u64;

    fn view_market(&self, market_address: &Address) -> MarketViewAll;
}

impl DropsetTestHelper for MolluskContext<HashMap<Address, Account>> {
    fn get_token_balance(&self, user: &Address, token_mint: &Address) -> u64 {
        let account_store = self.account_store.borrow();

        let user_ata = get_associated_token_address(user, token_mint);

        let acc = account_store.get(&user_ata).unwrap_or_else(|| {
            panic!("User token account doesn't exist, user: {user}, token account: {user_ata}")
        });

        TokenAccount::unpack(&acc.data)
            .map(|mint| mint.amount)
            .expect("Should unpack token account")
    }

    fn view_market(&self, market_address: &Address) -> MarketViewAll {
        let account_store = self.account_store.borrow();

        let acc = account_store
            .get(market_address)
            .expect("Market address should exist in mollusk account store");
        try_market_view_all(&acc.data).expect("Account data isn't valid for a market account")
    }
}
