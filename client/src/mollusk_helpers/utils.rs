use dropset_interface::state::SYSTEM_PROGRAM_ID;
use solana_account::Account;
use solana_address::Address;

/// Create the data necessary to send to [mollusk_svm::MolluskContext] to mock a funded account.
pub fn create_mock_user_account(address: Address, lamport_balance: u64) -> (Address, Account) {
    (
        address,
        Account {
            lamports: lamport_balance,
            data: vec![],
            owner: SYSTEM_PROGRAM_ID,
            executable: false,
            rent_epoch: 0,
        },
    )
}
