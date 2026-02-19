//! Lightweight, nonblocking RPC client utilities for funding accounts, sending transactions,
//! and pretty-printing `dropset`-related transaction logs.

use std::collections::HashSet;

use anyhow::{
    bail,
    Context,
};
use itertools::Itertools;
use solana_address::Address;
use solana_client::nonblocking::rpc_client::RpcClient;
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
use transaction_parser::{
    client_rpc::{
        parse_transaction,
        ParsedTransaction,
    },
    events::dropset_event::DropsetEvent,
    ParseDropsetEvents,
};

use crate::{
    pretty::{
        instruction_error::PrettyInstructionError,
        transaction::PrettyTransaction,
    },
    print_kv,
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
                "http://localhost:8899".into(),
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
            client: RpcClient::new_with_commitment(url.into(), CommitmentConfig::confirmed()),
            config,
        }
    }

    pub async fn fund_account(&self, address: &Address) -> anyhow::Result<()> {
        fund(&self.client, address).await
    }

    pub async fn fund_new_account(&self) -> anyhow::Result<Keypair> {
        let kp = Keypair::new();
        fund(&self.client, &kp.pubkey()).await?;

        Ok(kp)
    }

    /// Sends and confirms a single signer transaction with the signer passed in as the payer and
    /// sole signer.
    /// Instructions that require multiple signers should not be used here as they will obviously
    /// fail.
    pub async fn send_single_signer(
        &self,
        signer: &Keypair,
        instructions: impl AsRef<[Instruction]>,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        self.send_and_confirm_txn(signer, &[signer], instructions.as_ref())
            .await
    }

    /// Sends and confirms a transaction using [Self::config]. This automatically interpolates the
    /// payer into the signers array if they're included as a signer in any of the instructions.
    pub async fn send_and_confirm_txn(
        &self,
        payer: &Keypair,
        signers: &[&Keypair],
        instructions: &[Instruction],
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        send_transaction_with_config(&self.client, payer, signers, instructions, &self.config).await
    }
}

const MAX_TRIES: u8 = 20;

pub const DEFAULT_FUND_AMOUNT: u64 = 10_000_000_000;

async fn fund(rpc: &RpcClient, address: &Address) -> anyhow::Result<()> {
    let airdrop_signature: Signature = rpc
        .request_airdrop(address, DEFAULT_FUND_AMOUNT)
        .await
        .context("Failed to request airdrop")?;

    let mut i = 0;
    // Wait for airdrop confirmation.
    while !rpc
        .confirm_transaction(&airdrop_signature)
        .await
        .context("Couldn't confirm transaction")?
        && i < MAX_TRIES
    {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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
    pub program_id_filter: HashSet<Address>,
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

/// A parsed transaction together with all `DropsetEvent`s derived from it.
///
/// This bundles the decoded transaction data with the events extracted from
/// its execution logs, making it easier for callers to work with both in one
/// value.
pub struct ParsedTransactionWithEvents {
    /// The parsed representation of the confirmed transaction.
    pub parsed_transaction: ParsedTransaction,
    /// All `DropsetEvent`s parsed in the transaction.
    pub events: Vec<DropsetEvent>,
}

async fn send_transaction_with_config(
    rpc: &RpcClient,
    payer: &Keypair,
    signers: &[&Keypair],
    instructions: &[Instruction],
    config: &SendTransactionConfig,
) -> anyhow::Result<ParsedTransactionWithEvents> {
    let bh = rpc
        .get_latest_blockhash()
        .await
        .or(Err(()))
        .expect("Should be able to get blockhash.");

    let final_instructions: &[Instruction] = &[
        config.compute_budget.map_or(vec![], |budget| {
            vec![
                ComputeBudgetInstruction::set_compute_unit_limit(budget),
                ComputeBudgetInstruction::set_compute_unit_price(1),
            ]
        }),
        instructions.to_vec(),
    ]
    .concat();

    let msg = Message::new(final_instructions, Some(&payer.pubkey()));

    let mut tx = Transaction::new_unsigned(msg);
    tx.try_sign(
        &[std::iter::once(payer)
            .chain(signers.iter().cloned())
            .collect::<Vec<_>>()]
        .concat(),
        bh,
    )
    .expect("Should sign");

    let res = rpc.send_and_confirm_transaction(&tx).await;
    match res {
        Ok(signature) => {
            let encoded = fetch_transaction_json(rpc, signature).await?;
            let parsed_transaction = parse_transaction(encoded).expect("Should parse transaction");
            let dropset_events = parsed_transaction
                .instructions
                .iter()
                .flat_map(|outer| {
                    outer.inner_instructions.iter().flat_map(|inner_ixn| {
                        inner_ixn
                            .parse_events()
                            .expect("Should be able to parse events")
                    })
                })
                .collect_vec();

            if matches!(config.debug_logs, Some(true)) {
                print!(
                    "{}",
                    PrettyTransaction {
                        sender: payer.pubkey(),
                        signature,
                        indent_size: 2,
                        transaction: &parsed_transaction,
                        instruction_filter: &config.program_id_filter,
                    }
                );

                for event in dropset_events.iter() {
                    println!("{event:?}");
                }
            }

            Ok(ParsedTransactionWithEvents {
                parsed_transaction,
                events: dropset_events,
            })
        }
        Err(error) => {
            PrettyInstructionError::new(&error, final_instructions).inspect(|err| {
                print!("{err}");
                print_kv!("Payer", payer.pubkey(), LogColor::Error);
            });
            Err(error).context("Failed transaction submission")
        }
    }
}

async fn fetch_transaction_json(
    rpc: &RpcClient,
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
    .await
    .context("Should be able to fetch transaction with config")
}

/// Checks if an account at the given address exists on-chain.
pub async fn account_exists(rpc: &RpcClient, address: &Address) -> anyhow::Result<bool> {
    Ok(rpc
        .get_account_with_commitment(address, CommitmentConfig::confirmed())
        .await
        .context("Couldn't retrieve account data")?
        .value
        .is_some())
}
