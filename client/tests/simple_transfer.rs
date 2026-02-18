use client::mollusk_helpers::new_dropset_mollusk_context;
use dropset_interface::state::SYSTEM_PROGRAM_ID;
use solana_account::Account;
use solana_address::Address;
use solana_system_interface::instruction::transfer;

#[test]
fn simple_transfer() {
    const LAMPORTS: u64 = 10_000_000;
    let alice_address = Address::new_unique();
    let alice_account = Account::new(LAMPORTS, 0, &SYSTEM_PROGRAM_ID);
    let bob_address = Address::new_unique();
    let bob_account = Account::new(LAMPORTS, 0, &SYSTEM_PROGRAM_ID);
    let mollusk = new_dropset_mollusk_context(vec![
        (alice_address, alice_account.clone()),
        (bob_address, bob_account),
    ]);

    let get_account = |address: &Address| mollusk.account_store.borrow().get(address).cloned();

    let alice_before = get_account(&alice_address);
    let bob_before = get_account(&bob_address);
    assert!(alice_before.is_some());
    assert!(bob_before.is_some());
    assert_eq!(alice_before.unwrap().lamports, LAMPORTS);
    assert_eq!(bob_before.unwrap().lamports, LAMPORTS);

    // Transfer half of alice's lamports to bob.
    let send_to_bob = transfer(&alice_address, &bob_address, LAMPORTS / 2);
    assert!(mollusk
        .process_instruction(&send_to_bob)
        .program_result
        .is_ok());

    let alice_after = get_account(&alice_address).unwrap();
    let bob_after = get_account(&bob_address).unwrap();
    assert_eq!(alice_after.lamports, LAMPORTS / 2);
    assert_eq!(bob_after.lamports, LAMPORTS + LAMPORTS / 2);
}
