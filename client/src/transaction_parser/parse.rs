use solana_sdk::{
    message::MessageHeader,
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer,
    EncodedConfirmedTransactionWithStatusMeta,
    UiInnerInstructions,
    UiInstruction,
    UiMessage,
    UiParsedMessage,
    UiRawMessage,
};

use crate::transaction_parser::{
    parsed_account::ParsedAccount,
    parsed_instruction::{
        ParsedInnerInstruction,
        ParsedInstruction,
    },
    parsed_transaction::ParsedTransaction,
    ParsedAccounts,
};

// Re-export the main parsing entry function for clarity.
pub fn parse_transaction(
    encoded_with_meta: EncodedConfirmedTransactionWithStatusMeta,
) -> Result<ParsedTransaction, anyhow::Error> {
    ParsedTransaction::from_encoded_transaction(encoded_with_meta)
}

pub fn parse_inner_instructions(
    inner_instructions: OptionSerializer<Vec<UiInnerInstructions>>,
    parsed_accounts: &ParsedAccounts,
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
                        parsed_accounts,
                    ),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub fn parse_versioned_transaction(
    versioned: VersionedTransaction,
    addresses: &[Pubkey],
) -> (Vec<ParsedInstruction>, ParsedAccounts) {
    let keys = [versioned.message.static_account_keys(), addresses].concat();
    let parsed_accounts = parse_accounts_from_header(&keys, versioned.message.header());
    let parsed_instructions = versioned
        .message
        .instructions()
        .iter()
        .map(|ixn| ParsedInstruction::from_compiled_instruction(ixn, &parsed_accounts))
        .collect();
    (parsed_instructions, parsed_accounts)
}

/// Groups accounts by their read/write and signer/non-signer status according to the info in a
/// transaction's [`MessageHeader`]
///
/// ```text
/// where:
///  - n  = accounts.len()
///  - wr = write
///  - ro = read only
///
/// 0 ------------------------------------------ all accounts -------------------------------------n
/// |-------------------- signers --------------------|---------------- non_signers ---------------|
/// |------ wr_signers ------|------ ro_signers ------|--- wr_non_signers ---|--- ro_non_signers --|
/// 0----------------------- a ---------------------- b -------------------- c --------------------n
/// ```
pub fn parse_accounts_from_header(
    account_keys: &[Pubkey],
    header: &MessageHeader,
) -> ParsedAccounts {
    // Total number of signed accounts.
    let n_signers = header.num_required_signatures as usize;
    // The number of readonly signed accounts.
    let ro_signers = header.num_readonly_signed_accounts as usize;
    // The number of readonly, unsigned accounts.
    let ro_non_signers = header.num_readonly_unsigned_accounts as usize;

    let a = n_signers - ro_signers;
    let b = n_signers;
    let c = account_keys.len() - ro_non_signers;
    let d = account_keys.len();

    account_keys
        .iter()
        .enumerate()
        .map(|(ref i, pubkey)| {
            let (writable, signer) = match i {
                i if (0..a).contains(i) => (true, true),
                i if (a..b).contains(i) => (false, true),
                i if (b..c).contains(i) => (true, false),
                i if (c..d).contains(i) => (false, false),
                _ => unreachable!(),
            };
            ParsedAccount {
                pubkey: *pubkey,
                writable,
                signer,
            }
        })
        .collect()
}

pub fn parse_ui_message(
    ui_message: UiMessage,
    addresses: &[Pubkey],
) -> (Vec<ParsedInstruction>, ParsedAccounts) {
    let addresses_copied = addresses.iter().copied();
    match ui_message {
        UiMessage::Parsed(UiParsedMessage {
            account_keys,
            instructions,
            ..
        }) => {
            let parsed_accounts = account_keys
                .into_iter()
                .map(ParsedAccount::from)
                .collect::<ParsedAccounts>();

            let parsed_instructions = instructions
                .iter()
                .map(|ixn| ParsedInstruction::from_ui_instruction(ixn, &parsed_accounts))
                .collect::<Vec<_>>();
            (parsed_instructions, parsed_accounts)
        }
        UiMessage::Raw(UiRawMessage {
            account_keys,
            instructions,
            header,
            ..
        }) => {
            let keys = account_keys
                .iter()
                .map(|acc| Pubkey::from_str_const(acc))
                .chain(addresses_copied)
                .collect::<Vec<_>>();

            let parsed_accounts = parse_accounts_from_header(&keys, &header);

            let parsed_instructions = instructions
                .into_iter()
                .map(|ixn| {
                    ParsedInstruction::from_ui_instruction(
                        &UiInstruction::Compiled(ixn),
                        &parsed_accounts,
                    )
                })
                .collect();
            (parsed_instructions, parsed_accounts)
        }
    }
}
