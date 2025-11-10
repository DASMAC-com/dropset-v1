//! Lightweight RPC client utilities for funding accounts, sending transactions,
//! and pretty-printing `dropset`-related transaction logs.

use std::collections::HashSet;

use anyhow::{
    bail,
    Context,
};
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_sdk::{
    message::{
        Instruction,
        Message,
    },
    pubkey::Pubkey,
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
    pretty::{
        instruction_error::PrettyInstructionError,
        transaction::PrettyTransaction,
    },
    print_kv,
    transaction_parser::parse_transaction,
    LogColor,
};

pub struct CustomRpcClient {
    pub client: RpcClient,
    pub config: SendTransactionConfig,
}

impl Default for CustomRpcClient {
    fn default() -> Self {
        CustomRpcClient {
            client: RpcClient::new_with_commitment(
                "http://localhost:8899",
                CommitmentConfig::confirmed(),
            ),
            config: Default::default(),
        }
    }
}

impl CustomRpcClient {
    pub fn new(client: Option<RpcClient>, config: Option<SendTransactionConfig>) -> Self {
        match (client, config) {
            (Some(client), Some(config)) => Self { client, config },
            (client, config) => {
                let CustomRpcClient {
                    client: default_client,
                    config: default_config,
                } = Default::default();
                Self {
                    client: client.unwrap_or(default_client),
                    config: config.unwrap_or(default_config),
                }
            }
        }
    }

    pub fn new_from_url(url: &str, config: SendTransactionConfig) -> Self {
        CustomRpcClient {
            client: RpcClient::new_with_commitment(url, CommitmentConfig::confirmed()),
            config,
        }
    }

    pub async fn fund_account(&self, account: &Pubkey) -> anyhow::Result<()> {
        fund(&self.client, account).await
    }

    pub async fn fund_new_account(&self) -> anyhow::Result<Keypair> {
        let kp = Keypair::new();
        fund(&self.client, &kp.pubkey()).await?;

        Ok(kp)
    }

    pub async fn send_and_confirm_txn(
        &self,
        payer: &Keypair,
        signers: &[&Keypair],
        instructions: &[Instruction],
    ) -> anyhow::Result<Signature> {
        send_transaction_with_config(&self.client, payer, signers, instructions, &self.config).await
    }
}

const MAX_TRIES: u8 = 20;

async fn fund(rpc: &RpcClient, account: &Pubkey) -> anyhow::Result<()> {
    let airdrop_signature: Signature = rpc
        .request_airdrop(account, 10_000_000_000)
        .context("Failed to request airdrop")?;

    let mut i = 0;
    // Wait for airdrop confirmation.
    while !rpc
        .confirm_transaction(&airdrop_signature)
        .context("Couldn't confirm transaction")?
        && i < MAX_TRIES
    {
        std::thread::sleep(std::time::Duration::from_millis(500));
        i += 1;
    }

    if i == MAX_TRIES {
        bail!("Airdrop did not land.");
    }

    Ok(())
}

#[derive(Clone)]
pub struct SendTransactionConfig {
    pub compute_budget: Option<u32>,
    pub debug_logs: Option<bool>,
    pub program_id_filter: HashSet<Pubkey>,
}

impl Default for SendTransactionConfig {
    fn default() -> Self {
        SendTransactionConfig {
            compute_budget: Default::default(),
            debug_logs: Some(true),
            program_id_filter: HashSet::new(),
        }
    }
}

async fn send_transaction_with_config(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    instructions: &[Instruction],
    config: &SendTransactionConfig,
) -> anyhow::Result<Signature> {
    let bh = rpc
        .get_latest_blockhash()
        .or(Err(()))
        .expect("Should be able to get blockhash.");

    let msg = Message::new(
        &[
            config.compute_budget.map_or(vec![], |budget| {
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
        Ok(signature) => {
            if matches!(config.debug_logs, Some(true)) {
                let encoded = fetch_transaction_json(rpc, signature).await?;
                match parse_transaction(encoded) {
                    Ok(ref transaction) => {
                        print!(
                            "{}",
                            PrettyTransaction {
                                sender: payer.pubkey(),
                                signature,
                                indent_size: 2,
                                transaction,
                                instruction_filter: &config.program_id_filter,
                            }
                        );
                    }
                    Err(e) => {
                        eprintln!("{e}");
                    }
                }
            }
            Ok(signature)
        }
        Err(error) => {
            PrettyInstructionError::new(&error, instructions).inspect(|err| {
                print!("{err}");
                print_kv!("Payer", payer.pubkey(), LogColor::Error);
            });
            Err(error).context("Failed transaction submission")
        }
    }
}

async fn fetch_transaction_json(
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
