mod global_fixture;
mod market_fixture;
mod mint_fixture;
mod test_fixture;
mod token_account_fixture;

use std::{
    cell::{
        RefCell,
        RefMut,
    },
    rc::Rc,
};

pub use global_fixture::*;
use manifest::program::ManifestInstruction;
pub use market_fixture::*;
pub use mint_fixture::*;
use solana_program::hash::Hash;
use solana_program_test::{
    BanksClientError,
    ProgramTestContext,
};
use solana_sdk::{
    account::Account,
    instruction::{
        AccountMeta,
        Instruction,
        InstructionError,
    },
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::{
        Transaction,
        TransactionError,
    },
};
pub use test_fixture::*;
pub use token_account_fixture::*;

#[derive(PartialEq)]
pub enum Token {
    USDC = 0,
    SOL = 1,
}

pub const SOL_UNIT_SIZE: u64 = 1_000_000_000;
pub const USDC_UNIT_SIZE: u64 = 1_000_000;

pub async fn get_and_deserialize<T: Pack>(
    context: Rc<RefCell<ProgramTestContext>>,
    pubkey: Pubkey,
) -> T {
    let context: RefMut<ProgramTestContext> = context.borrow_mut();
    loop {
        let account_or: Result<Option<Account>, BanksClientError> =
            context.banks_client.get_account(pubkey).await;
        if account_or.is_err() {
            continue;
        }
        let account_opt: Option<Account> = account_or.unwrap();
        if account_opt.is_none() {
            continue;
        }
        return T::unpack_unchecked(account_opt.unwrap().data.as_slice()).unwrap();
    }
}

pub async fn send_tx_with_retry(
    context: Rc<RefCell<ProgramTestContext>>,
    instructions: &[Instruction],
    payer: Option<&Pubkey>,
    signers: &[&Keypair],
) -> Result<(), BanksClientError> {
    let mut context: RefMut<ProgramTestContext> = context.borrow_mut();

    loop {
        let blockhash_or: Result<Hash, std::io::Error> = context.get_new_latest_blockhash().await;
        if blockhash_or.is_err() {
            continue;
        }
        let tx: Transaction =
            Transaction::new_signed_with_payer(instructions, payer, signers, blockhash_or.unwrap());
        let result: Result<(), BanksClientError> =
            context.banks_client.process_transaction(tx).await;
        if result.is_ok() {
            break;
        }
        let error: BanksClientError = result.err().unwrap();
        match error {
            BanksClientError::RpcError(_rpc_err) => {
                continue;
            }
            BanksClientError::Io(_io_err) => {
                continue;
            }
            BanksClientError::TransactionError(TransactionError::InstructionError(
                idx,
                InstructionError::ProgramFailedToComplete,
            )) => {
                // The `ProgramFailedToComplete` error seems to show up only if there is no
                // `ComputeBudgetInstruction` passed to the program. If it is passed (and exceeded),
                // there is an explicit `ComputationalBudgetExceeded` error.
                // Since this behavior is fairly confusing and opaque, this warning is here.
                eprintln!(
                    "send_tx_with_retry: instruction {idx} failed with \
                     ProgramFailedToComplete (possibly exceeded compute budget)"
                );
                return Err(error);
            }
            _ => {
                println!("Unexpected error: {:?}", error);
                return Err(error);
            }
        }
    }
    Ok(())
}

pub async fn expand_market(
    context: Rc<RefCell<ProgramTestContext>>,
    market: &Pubkey,
    num_free_blocks: u32,
) -> Result<(), BanksClientError> {
    use solana_program::system_program;

    let payer_keypair = context.borrow().payer.insecure_clone();
    let payer = payer_keypair.pubkey();

    let expand_ix = Instruction {
        program_id: manifest::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(*market, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: [
            ManifestInstruction::Expand.to_vec(),
            num_free_blocks.to_le_bytes().to_vec(),
        ]
        .concat(),
    };

    send_tx_with_retry(
        Rc::clone(&context),
        &[expand_ix],
        Some(&payer),
        &[&payer_keypair],
    )
    .await?;

    Ok(())
}
