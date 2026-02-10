use std::{
    collections::HashMap,
    path::PathBuf,
};

use mollusk_svm::{
    Mollusk,
    MolluskContext,
};
use solana_account::Account;
use solana_address::Address;

/// Converts an input deploy file to a program name used by the [`Mollusk::new`] function.
///
/// Requires the full file name; for example, `dropset.so` would return the absolute path version of
/// `../target/deploy/dropset`, which is exactly what [`Mollusk::new`] expects.
pub fn deploy_file_to_program_name(program_name: &str) -> String {
    PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("target/deploy/")
        .join(program_name)
        .canonicalize()
        .map(|p| {
            p.to_str()
                .expect("Path should convert to a &str")
                .strip_suffix(".so")
                .expect("Deploy file should have an `.so` suffix")
                .to_string()
        })
        .expect("Should create relative target/deploy/ path")
}

/// Creates and returns a [`MolluskContext`] with the dropset program and the passed accounts
/// already created.
pub fn new_dropset_mollusk_context(
    accounts: Vec<(Address, Account)>,
) -> MolluskContext<HashMap<Address, Account>> {
    let mollusk = Mollusk::new(&dropset::ID, &deploy_file_to_program_name("dropset.so"));

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::e2e_helpers::mollusk::deploy_file_to_program_name;

    #[test]
    fn dropset_program_path() {
        let dropset = deploy_file_to_program_name("dropset.so");
        assert!(dropset.ends_with("dropset"));

        // Ensure the program deploy path is a valid file.
        assert!(PathBuf::from([dropset.as_str(), ".so"].concat()).is_file());
    }
}
