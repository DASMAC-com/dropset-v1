use std::collections::HashMap;

use mollusk_svm::{
    Mollusk,
    MolluskContext,
};
use solana_account::Account;
use solana_address::Address;

/// Creates and returns a [`MolluskContext`] with the dropset program and the passed accounts
/// already created.
pub fn new_dropset_mollusk_context(
    accounts: Vec<(Address, Account)>,
) -> MolluskContext<HashMap<Address, Account>> {
    let mollusk = Mollusk::new(&dropset::ID, "../target/deploy/dropset");

    // Create mollusk context with the simple hashmap implementation for the AccountStore.
    let context = mollusk.with_context(HashMap::new());

    // Create each account passed in at its respective address using the specified account data.
    // This "funds" accounts in the sense that it will create the account with the specified
    // lamport balance in its account data.
    for (address, account) in accounts {
        context.account_store.borrow_mut().insert(address, account);
    }

    context
}
