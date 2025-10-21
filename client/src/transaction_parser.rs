use solana_sdk::{
    bs58,
    clock::UnixTimestamp,
    message::compiled_instruction::CompiledInstruction,
    pubkey::Pubkey,
    transaction::{
        TransactionVersion,
        VersionedTransaction,
    },
};
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    EncodedConfirmedTransactionWithStatusMeta,
    EncodedTransaction,
    UiInnerInstructions,
    UiInstruction,
    UiMessage,
    UiParsedInstruction,
    UiParsedMessage,
    UiRawMessage,
    UiTransaction,
    UiTransactionTokenBalance,
};
use solana_transaction_status_client_types::UiTransactionError;

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

#[derive(Debug)]
pub struct ParsedOuterInstruction {
    pub outer_instruction: ParsedInstruction,
    pub inner_instructions: Vec<ParsedInstruction>,
}

#[derive(Debug)]
pub struct ParsedInstruction {
    pub program_id: Pubkey,
    pub accounts: Vec<Pubkey>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct ParsedInnerInstruction {
    pub parent_index: u8,
    pub inner_instruction: ParsedInstruction,
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

        let addresses = match meta.loaded_addresses {
            OptionSerializer::Some(addresses) => [addresses.writable, addresses.readonly]
                .concat()
                .iter()
                .map(|s| Pubkey::from_str_const(s))
                .collect::<Vec<_>>(),
            _ => vec![],
        };

        let (outer_instructions, program_ids) = match transaction.transaction {
            EncodedTransaction::Json(UiTransaction {
                signatures: _,
                message,
            }) => parse_ui_message(message, &addresses),
            encoded => {
                let versioned = encoded.decode().expect("Should decode transaction");
                parse_versioned_transaction(versioned, &addresses)
            }
        };

        let inner_instructions = parse_inner_instructions(meta.inner_instructions, &program_ids);

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

pub fn parse_inner_instructions(
    inner_instructions: OptionSerializer<Vec<UiInnerInstructions>>,
    program_ids: &[Pubkey],
) -> Vec<ParsedInnerInstruction> {
    inner_instructions
        .unwrap_or(vec![])
        .iter()
        .flat_map(|inner| {
            inner
                .instructions
                .iter()
                .map(|ui_instruction| ParsedInnerInstruction {
                    parent_index: inner.index,
                    inner_instruction: ParsedInstruction::from_ui_instruction(
                        ui_instruction,
                        program_ids,
                    ),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn parse_versioned_transaction(
    versioned: VersionedTransaction,
    addresses: &[Pubkey],
) -> (Vec<ParsedInstruction>, Vec<Pubkey>) {
    let program_ids = [versioned.message.static_account_keys(), addresses].concat();
    let instructions = versioned
        .message
        .instructions()
        .iter()
        .map(|ixn| ParsedInstruction::from_compiled_instruction(ixn, &program_ids))
        .collect();
    (instructions, program_ids)
}

pub fn parse_ui_message(
    ui_message: UiMessage,
    addresses: &[Pubkey],
) -> (Vec<ParsedInstruction>, Vec<Pubkey>) {
    let addresses_copied = addresses.iter().copied();
    match ui_message {
        UiMessage::Parsed(UiParsedMessage {
            account_keys,
            instructions,
            ..
        }) => {
            let program_ids = account_keys
                .iter()
                .map(|acc| Pubkey::from_str_const(&acc.pubkey))
                .chain(addresses_copied)
                .collect::<Vec<_>>();

            let parsed = instructions
                .iter()
                .map(|ixn| ParsedInstruction::from_ui_instruction(ixn, &program_ids))
                .collect::<Vec<_>>();
            (parsed, program_ids)
        }
        UiMessage::Raw(UiRawMessage {
            account_keys,
            instructions,
            ..
        }) => {
            let program_ids = account_keys
                .iter()
                .map(|acc| Pubkey::from_str_const(acc))
                .chain(addresses_copied)
                .collect::<Vec<_>>();
            let parsed = instructions
                .into_iter()
                .map(|ixn| {
                    ParsedInstruction::from_ui_instruction(
                        &UiInstruction::Compiled(ixn),
                        &program_ids,
                    )
                })
                .collect();

            (parsed, program_ids)
        }
    }
}

impl ParsedInstruction {
    pub fn from_compiled_instruction(
        instruction: &CompiledInstruction,
        program_ids: &[Pubkey],
    ) -> Self {
        Self {
            program_id: instruction.program_id(program_ids).into(),
            accounts: instruction
                .accounts
                .iter()
                .map(|i| program_ids[*i as usize])
                .collect(),
            data: instruction.data.clone(),
        }
    }

    pub fn from_ui_instruction(instruction: &UiInstruction, program_ids: &[Pubkey]) -> Self {
        match instruction {
            UiInstruction::Compiled(compiled) => Self {
                program_id: program_ids[compiled.program_id_index as usize],
                accounts: compiled
                    .accounts
                    .iter()
                    .map(|i| program_ids[*i as usize])
                    .collect(),
                data: bs58::decode(&compiled.data)
                    .into_vec()
                    .expect("Should base58 decode"),
            },
            UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(decoded)) => Self {
                program_id: Pubkey::from_str_const(&decoded.program_id),
                accounts: decoded
                    .accounts
                    .iter()
                    .map(|s| Pubkey::from_str_const(s))
                    .collect::<Vec<_>>(),
                data: bs58::decode(&decoded.data)
                    .into_vec()
                    .expect("Should base58 decode"),
            },
            // It's unclear how to parse already parsed ui transactions.
            UiInstruction::Parsed(UiParsedInstruction::Parsed(_)) => unimplemented!(),
        }
    }
}
