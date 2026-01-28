use client::e2e_helpers::mollusk::new_dropset_mollusk_context;
use dropset_interface::state::SYSTEM_PROGRAM_ID;
use solana_account::Account;
use solana_address::Address;
use solana_instruction::{
    AccountMeta,
    Instruction,
};
use solana_system_interface::instruction::SystemInstruction;

#[test]
fn instruction_data_len_cost() {
    const LAMPORTS: u64 = 10_000_000;
    let alice_address = Address::new_unique();
    let bob_address = Address::new_unique();
    let mollusk = new_dropset_mollusk_context(vec![
        (alice_address, Account::new(LAMPORTS, 0, &SYSTEM_PROGRAM_ID)),
        (bob_address, Account::new(LAMPORTS, 0, &SYSTEM_PROGRAM_ID)),
    ]);

    let get_account = |address: &Address| mollusk.account_store.borrow().get(address).cloned();

    let alice_before = get_account(&alice_address);
    let bob_before = get_account(&bob_address);
    assert_eq!(alice_before.unwrap().lamports, LAMPORTS);
    assert_eq!(bob_before.unwrap().lamports, LAMPORTS);

    let transfer_instruction_data = bincode::serialize(&SystemInstruction::Transfer {
        lamports: LAMPORTS / 2,
    })
    .unwrap();

    let account_metas = vec![
        AccountMeta::new(alice_address, true),
        AccountMeta::new(bob_address, true),
    ];

    let transfer = mollusk.process_instruction(&Instruction::new_with_bytes(
        SYSTEM_PROGRAM_ID,
        &transfer_instruction_data,
        account_metas.clone(),
    ));

    let transfer_with_extra_data = mollusk.process_instruction(&Instruction::new_with_bytes(
        SYSTEM_PROGRAM_ID,
        &[transfer_instruction_data, [0u8; 1000].to_vec()].concat(),
        account_metas,
    ));

    assert!(transfer.program_result.is_ok());
    assert!(transfer_with_extra_data.program_result.is_ok());
    assert_eq!(
        transfer.compute_units_consumed,
        transfer_with_extra_data.compute_units_consumed
    );
}
