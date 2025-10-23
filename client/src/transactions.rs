use anyhow::Context;
use colored::Colorize;
use dropset_interface::{
    error::DropsetError,
    instructions::DropsetInstruction,
};
use solana_client::{
    client_error::{
        ClientError,
        ClientErrorKind,
    },
    rpc_client::RpcClient,
    rpc_response::RpcSimulateTransactionResult,
};
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::{
    message::{
        Instruction,
        Message,
    },
    signature::{
        Keypair,
        Signature,
        Signer,
    },
    transaction::Transaction,
};
use solana_transaction_status::{
    EncodedConfirmedTransactionWithStatusMeta,
    UiTransactionEncoding,
};

use crate::{
    logs::{
        log_error,
        log_info,
        log_success,
        LogColor,
    },
    pretty::transaction::PrettyTransaction,
    transaction_parser::ParsedTransaction,
};

pub async fn fund_account(rpc: &RpcClient, keypair: Option<Keypair>) -> anyhow::Result<Keypair> {
    let payer = match keypair {
        Some(kp) => kp,
        None => Keypair::new(),
    };

    let airdrop_signature = rpc
        .request_airdrop(&payer.pubkey(), 10_000_000_000)
        .context("Failed to request airdrop")?;

    let mut i = 0;
    // Wait for airdrop confirmation.
    while !rpc
        .confirm_transaction(&airdrop_signature)
        .context("Couldn't confirm transaction")?
        && i < 10
    {
        std::thread::sleep(std::time::Duration::from_millis(500));
        i += 1;
    }

    Ok(payer)
}

pub async fn send_transaction(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    instructions: &[Instruction],
) -> anyhow::Result<Signature> {
    send_transaction_with_config(rpc, payer, signers, instructions, None).await
}

pub struct SendTransactionConfig {
    pub compute_budget: Option<u32>,
    pub debug_logs: Option<bool>,
}

impl Default for SendTransactionConfig {
    fn default() -> Self {
        SendTransactionConfig {
            compute_budget: Default::default(),
            debug_logs: Some(true),
        }
    }
}

pub async fn send_transaction_with_config(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    instructions: &[Instruction],
    config: Option<SendTransactionConfig>,
) -> anyhow::Result<Signature> {
    let bh = rpc
        .get_latest_blockhash()
        .or(Err(()))
        .expect("Should be able to get blockhash.");

    let SendTransactionConfig {
        compute_budget,
        debug_logs,
    } = config.unwrap_or_default();

    let msg = Message::new(
        &[
            compute_budget.map_or(vec![], |budget| {
                vec![
                    ComputeBudgetInstruction::set_compute_unit_limit(budget),
                    ComputeBudgetInstruction::set_compute_unit_price(1),
                ]
            }),
            instructions.to_vec(),
        ]
        .concat(),
        Some(&payer.pubkey()),
    );

    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(
        &[std::iter::once(payer)
            .chain(signers.iter().cloned())
            .collect::<Vec<_>>()]
        .concat(),
        bh,
    )
    .expect("Should sign");

    let res = rpc.send_and_confirm_transaction(&tx);
    match res {
        Ok(sig) => {
            if matches!(debug_logs, Some(true)) {
                println!();
                let sender_info = format!("{}: {}", "sender".color(LogColor::Gray), payer.pubkey());
                let tx_info = format! {"{sig}\n{sender_info}"};
                log_success("Signature", tx_info);
                let encoded = get_transaction_json(rpc, sig).await?;
                let parsed = ParsedTransaction::from_encoded_transaction(encoded);
                parsed.inspect(|transaction| {
                    println!(
                        "\n{}",
                        PrettyTransaction {
                            indent: 2,
                            transaction,
                        }
                    );
                });
            }
            Ok(sig)
        }
        Err(error) => {
            log_instruction_error(&error, instructions);
            log_info("Payer", payer.pubkey());

            Err(error).context("Failed transaction submission")
        }
    }
}

pub async fn get_transaction_json(
    rpc: &solana_client::rpc_client::RpcClient,
    sig: Signature,
) -> anyhow::Result<EncodedConfirmedTransactionWithStatusMeta> {
    rpc.get_transaction_with_config(
        &sig,
        solana_client::rpc_config::RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Json),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        },
    )
    .context("Should be able to fetch transaction with config")
}

pub fn log_instruction_error(error: &ClientError, instructions: &[Instruction]) {
    use solana_client::rpc_request::{
        RpcError::RpcResponseError,
        RpcResponseErrorData,
    };
    use solana_instruction_error::InstructionError;
    use solana_transaction_error::TransactionError;

    let kind = error.kind();
    if let ClientErrorKind::RpcError(RpcResponseError {
        data:
            RpcResponseErrorData::SendTransactionPreflightFailure(RpcSimulateTransactionResult {
                err: Some(ui_err),
                ..
            }),
        ..
    }) = kind
    {
        if let TransactionError::InstructionError(ixn_idx, ixn_error) = ui_err.clone().into() {
            let instruction = instructions
                .get(ixn_idx as usize)
                .expect("Index should be valid");

            let tag = instruction.data[0];

            match ixn_error {
                InstructionError::Custom(code) => {
                    if instruction.program_id.as_ref() == dropset::ID {
                        let error = DropsetError::from_repr(code as u8).expect("Should be valid");
                        let tag = DropsetInstruction::try_from_u8(tag, || anyhow::Error::msg(""))
                            .expect("Should be valid");
                        let msg = format!("({tag}, {error})");
                        log_error("Dropset error", msg);
                    }
                }
                _ => log_error("Generic error", error),
            }
        }
    }
}
