//! High-level parsed transaction type that aggregates instructions, logs, balances, errors, and
//! compute usage.

use std::str::FromStr;

use solana_sdk::{
    clock::UnixTimestamp,
    pubkey::Pubkey,
    signature::Signature,
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

use crate::client_rpc::{
    add_infos_to_outer_instructions,
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
    GroupedParsedLogs,
};

#[derive(Debug)]
pub struct ParsedTransaction {
    pub version: Option<i8>,
    pub signature: Signature,
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
    ) -> Result<Self, anyhow::Error> {
        let EncodedConfirmedTransactionWithStatusMeta {
            slot,
            block_time,
            transaction,
        } = encoded;

        let meta = transaction
            .meta
            .ok_or(anyhow::Error::msg("Expected transaction meta"))?;
        let log_messages = meta.log_messages.unwrap_or(vec![]);
        let compute_infos = parse_logs_for_compute(&log_messages).expect("Should parse");

        let addresses = match meta.loaded_addresses {
            OptionSerializer::Some(addresses) => [addresses.writable, addresses.readonly]
                .concat()
                .iter()
                .map(|s| Pubkey::from_str_const(s))
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        let (outer_instructions, parsed_accounts, signature) = match transaction.transaction {
            EncodedTransaction::Json(UiTransaction {
                signatures,
                message,
            }) => {
                let (instructions, accounts) = parse_ui_message(message, &addresses);
                let signature =
                    Signature::from_str(&signatures[0]).expect("Should be a valid signature");
                (instructions, accounts, signature)
            }
            encoded => {
                let versioned: solana_sdk::transaction::VersionedTransaction =
                    encoded.decode().expect("Should decode transaction");
                parse_versioned_transaction(versioned, &addresses)
            }
        };

        let inner_instructions: Vec<ParsedInnerInstruction> =
            parse_inner_instructions(meta.inner_instructions, &parsed_accounts);

        Ok(Self {
            version: transaction.version.map(|v| match v {
                TransactionVersion::Number(v) => v as i8,
                _ => -1,
            }),
            signature,
            slot,
            block_time,
            err: meta.err,
            fee: meta.fee,
            pre_balances: meta.pre_balances,
            post_balances: meta.post_balances,
            instructions: Self::parse_outer_instructions(
                outer_instructions,
                inner_instructions,
                Some(compute_infos),
            )?,
            log_messages,
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
        maybe_compute_map: Option<Vec<GroupedParsedLogs>>,
    ) -> Result<Vec<ParsedOuterInstruction>, anyhow::Error> {
        // Group outers as a vec.
        let mut outers = outer_instructions
            .into_iter()
            .map(|outer| ParsedOuterInstruction {
                outer_instruction: outer,
                inner_instructions: vec![],
            })
            .collect::<Vec<_>>();

        // Push the inner instructions to their corresponding outer instruction vec.
        for inner in inner_instructions {
            outers
                .get_mut(inner.parent_index as usize)
                .expect("Parent index should exist")
                .inner_instructions
                .push(inner.inner_instruction);
        }

        if let Some(compute_map) = maybe_compute_map {
            add_infos_to_outer_instructions(&mut outers, compute_map)?;
        }

        Ok(outers)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs::File,
        io::BufReader,
    };

    use solana_sdk::pubkey::Pubkey;

    use crate::client_rpc::{
        parse_logs_for_compute,
        GroupedParsedLogs,
        ParsedLogs,
    };

    #[test]
    fn parse_goldens_happy_path() {
        let file = File::open("test_logs.json").expect("File should exist");
        let reader = BufReader::new(file);

        let map: HashMap<String, Vec<String>> =
            serde_json::from_reader(reader).unwrap_or_else(|_| HashMap::new());

        for (_signature, log_messages) in map {
            let res = parse_logs_for_compute(&log_messages);
            assert!(res.is_ok());
        }
    }

    /// Helper function to create the `ParsedLogs` for an inner/child instruction.
    fn child_logs(
        invocation_index: usize,
        program_id: &str,
        stack_height: usize,
        units: Option<u64>,
        allowed: Option<u64>,
        parent_index: usize,
        logs: Vec<&str>,
    ) -> ParsedLogs {
        ParsedLogs {
            invocation_index,
            program_id: Pubkey::from_str_const(program_id),
            stack_height,
            units_consumed: units,
            consumption_allowance: allowed,
            parent_index: Some(parent_index),
            program_logs: logs.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn parse_complex() {
        let complex_txn_sig = "5oQeU4AnZstnyuv77WMsgaDMRhgGnCrgEWC72pKGB2k3P3dqUHSDBGZWALbNDugCJEMBgy8pQnY8C87rP8oHFrZv";
        let file = File::open("test_logs.json").expect("File should exist");
        let reader = BufReader::new(file);

        let map: HashMap<String, Vec<String>> =
            serde_json::from_reader(reader).unwrap_or_else(|_| HashMap::new());

        let parsed_logs = map
            .get(complex_txn_sig)
            .and_then(|logs| parse_logs_for_compute(logs).ok())
            .expect("Should parse");

        const TEST_IDX: usize = 2;

        let expected: Vec<GroupedParsedLogs> = vec![
            // Setting the compute unit limit.
            GroupedParsedLogs {
                parent: ParsedLogs {
                    invocation_index: 0,
                    program_id: Pubkey::from_str_const(
                        "ComputeBudget111111111111111111111111111111",
                    ),
                    stack_height: 1,
                    units_consumed: None,
                    consumption_allowance: None,
                    parent_index: None,
                    program_logs: vec![],
                },
                children: vec![],
            },
            // Setting the compute unit price.
            GroupedParsedLogs {
                parent: ParsedLogs {
                    invocation_index: 1,
                    program_id: Pubkey::from_str_const(
                        "ComputeBudget111111111111111111111111111111",
                    ),
                    stack_height: 1,
                    units_consumed: None,
                    consumption_allowance: None,
                    parent_index: None,
                    program_logs: vec![],
                },
                children: vec![],
            },
            // The outer `TEST` program invocation, with inner children.
            GroupedParsedLogs {
                parent: ParsedLogs {
                    invocation_index: TEST_IDX,
                    program_id: dropset::ID.into(),
                    stack_height: 1,
                    units_consumed: Some(55834),
                    consumption_allowance: Some(1399700),
                    parent_index: None,
                    program_logs: vec![
                        "TEST program log 0".to_string(),
                        "TEST program log 1".to_string(),
                        "TEST program log 2".to_string(),
                        "TEST program log 3".to_string(),
                        "TEST program log 4".to_string(),
                    ],
                },
                #[rustfmt::skip]
                // Inner parsed logs for creating the associated token accounts for base and quote.
                children: vec![
                    child_logs(3, "11111111111111111111111111111111", 2, None, None, TEST_IDX, vec![]),
                    child_logs(4, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", 2, Some(21990), Some(1394597), TEST_IDX, vec!["Create", "Initialize the associated token account"]),
                    child_logs(5, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(1595), Some(1387622), TEST_IDX, vec!["Instruction: GetAccountDataSize"]),
                    child_logs(6, "11111111111111111111111111111111", 3, None, None, TEST_IDX, vec![]),
                    child_logs(7, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(1405), Some(1381009), TEST_IDX, vec!["Instruction: InitializeImmutableOwner", "Please upgrade to SPL Token 2022 for immutable owner support"]),
                    child_logs(8, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(4214), Some(1377125), TEST_IDX, vec!["Instruction: InitializeAccount3"]),
                    child_logs(9, "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", 2, Some(26490), Some(1370597), TEST_IDX, vec!["Create", "Initialize the associated token account"]),
                    child_logs(10, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(1595), Some(1359122), TEST_IDX, vec!["Instruction: GetAccountDataSize"]),
                    child_logs(11, "11111111111111111111111111111111", 3, None, None, TEST_IDX, vec![]),
                    child_logs(12, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(1405), Some(1352509), TEST_IDX, vec!["Instruction: InitializeImmutableOwner", "Please upgrade to SPL Token 2022 for immutable owner support"]),
                    child_logs(13, "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", 3, Some(4214), Some(1348625), TEST_IDX, vec!["Instruction: InitializeAccount3"]),
                ],
            },
        ];

        // Assert equality for expected and parsed values.
        for (expected_group, parsed_group) in expected.into_iter().zip(parsed_logs) {
            assert_eq!(expected_group.parent, parsed_group.parent);
            for (expected_child, parsed_child) in expected_group
                .children
                .into_iter()
                .zip(parsed_group.children)
            {
                assert_eq!(expected_child, parsed_child);
            }
        }
    }
}
