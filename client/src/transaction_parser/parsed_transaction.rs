use solana_sdk::{
    clock::UnixTimestamp,
    pubkey::Pubkey,
    transaction::TransactionVersion,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    EncodedConfirmedTransactionWithStatusMeta,
    EncodedTransaction,
    UiTransaction,
    UiTransactionTokenBalance,
};
use solana_transaction_status_client_types::UiTransactionError;

use crate::transaction_parser::{
    parse::{
        parse_inner_instructions,
        parse_ui_message,
        parse_versioned_transaction,
    },
    parse_logs_for_compute,
    parsed_instruction::{
        ParsedInnerInstruction,
        ParsedInstruction,
        ParsedOuterInstruction,
    },
};

#[derive(Debug)]
pub struct ParsedTransaction {
    pub version: Option<i8>,
    pub slot: u64,
    pub block_time: Option<UnixTimestamp>,
    pub err: Option<UiTransactionError>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub instructions: Vec<ParsedOuterInstruction>,
    pub log_messages: Vec<String>,
    pub pre_token_balances: Vec<UiTransactionTokenBalance>,
    pub post_token_balances: Vec<UiTransactionTokenBalance>,
    pub raw_compute_usage: Option<u64>,
}

impl ParsedTransaction {
    pub fn from_encoded_transaction(
        encoded: EncodedConfirmedTransactionWithStatusMeta,
    ) -> Option<Self> {
        let EncodedConfirmedTransactionWithStatusMeta {
            slot,
            block_time,
            transaction,
        } = encoded;

        let meta = transaction.meta?;
        let computes = parse_logs_for_compute(&meta);

        meta.log_messages
            .clone()
            .unwrap()
            .iter()
            .for_each(|l| println!("{l}"));

        let sorted = computes.clone();

        for i in 1..(sorted.len() * 2) + 1 {
            let invoke = sorted
                .iter()
                .find(|ixn| i as u8 == ixn.absolute_invoke_index);
            if let Some(invoke) = invoke {
                let indent = "    ".repeat(invoke.stack_height as usize);
                println!(
                    "{}Program {} invoke [{}]",
                    indent, invoke.program_id, invoke.stack_height
                );
            } else {
                let success = sorted.iter().find(|txn| i as u8 == txn.absolute_cu_index);
                assert!(success.is_some());
                let ixn = success.unwrap();
                let indent = "    ".repeat(ixn.stack_height as usize);
                println!(
                    "{}Program {} consumed {} of {} units",
                    indent, ixn.program_id, ixn.units_consumed, ixn.total_consumption
                );
            }
        }

        let addresses = match meta.loaded_addresses {
            OptionSerializer::Some(addresses) => [addresses.writable, addresses.readonly]
                .concat()
                .iter()
                .map(|s| Pubkey::from_str_const(s))
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        let (outer_instructions, parsed_accounts) = match transaction.transaction {
            EncodedTransaction::Json(UiTransaction {
                signatures: _,
                message,
            }) => parse_ui_message(message, &addresses),
            encoded => {
                let versioned = encoded.decode().expect("Should decode transaction");
                parse_versioned_transaction(versioned, &addresses)
            }
        };

        let inner_instructions: Vec<ParsedInnerInstruction> =
            parse_inner_instructions(meta.inner_instructions, &parsed_accounts);

        Some(Self {
            version: transaction.version.map(|v| match v {
                TransactionVersion::Number(v) => v as i8,
                _ => -1,
            }),
            slot,
            block_time,
            err: meta.err,
            fee: meta.fee,
            pre_balances: meta.pre_balances,
            post_balances: meta.post_balances,
            instructions: Self::parse_outer_instructions(outer_instructions, inner_instructions),
            log_messages: meta.log_messages.unwrap_or(vec![]),
            pre_token_balances: meta.pre_token_balances.unwrap_or(vec![]),
            post_token_balances: meta.post_token_balances.unwrap_or(vec![]),
            raw_compute_usage: match (meta.compute_units_consumed, meta.cost_units) {
                (OptionSerializer::Some(consumed), OptionSerializer::Some(units)) => {
                    Some(consumed * units)
                }
                _ => None,
            },
        })
    }

    fn parse_outer_instructions(
        outer_instructions: Vec<ParsedInstruction>,
        inner_instructions: Vec<ParsedInnerInstruction>,
    ) -> Vec<ParsedOuterInstruction> {
        let mut outers = outer_instructions
            .into_iter()
            .map(|outer| ParsedOuterInstruction {
                outer_instruction: outer,
                inner_instructions: vec![],
            })
            .collect::<Vec<_>>();

        for inner in inner_instructions {
            outers
                .get_mut(inner.parent_index as usize)
                .expect("Parent index should exist")
                .inner_instructions
                .push(inner.inner_instruction);
        }

        outers
    }
}
