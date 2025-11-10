//! Typed representations of outer and inner instructions, including decoded program IDs, accounts,
//! and data.

use solana_sdk::{
    bs58,
    message::compiled_instruction::CompiledInstruction,
    pubkey::Pubkey,
};
use solana_transaction_status::{
    UiInstruction,
    UiParsedInstruction,
};

use crate::transaction_parser::{
    ParsedAccounts,
    ParsedLogs,
};

#[derive(Debug)]
pub struct ParsedInstruction {
    pub program_id: Pubkey,
    pub accounts: ParsedAccounts,
    pub data: Vec<u8>,
    pub compute_info: Option<ParsedLogs>,
}

#[derive(Debug)]
pub struct ParsedOuterInstruction {
    pub outer_instruction: ParsedInstruction,
    pub inner_instructions: Vec<ParsedInstruction>,
}

#[derive(Debug)]
pub struct ParsedInnerInstruction {
    pub parent_index: u8,
    pub inner_instruction: ParsedInstruction,
}

impl ParsedInstruction {
    pub fn from_compiled_instruction(
        instruction: &CompiledInstruction,
        parsed_accounts: &ParsedAccounts,
    ) -> Self {
        Self {
            program_id: *instruction.program_id(&parsed_accounts.pubkeys()),
            accounts: instruction
                .accounts
                .iter()
                .map(|i| parsed_accounts[*i as usize])
                .collect(),
            data: instruction.data.clone(),
            compute_info: None,
        }
    }

    pub fn from_ui_instruction(
        instruction: &UiInstruction,
        parsed_accounts: &ParsedAccounts,
    ) -> Self {
        match instruction {
            UiInstruction::Compiled(compiled) => Self {
                program_id: parsed_accounts[compiled.program_id_index as usize].pubkey,
                accounts: compiled
                    .accounts
                    .iter()
                    .map(|i| parsed_accounts[*i as usize])
                    .collect(),
                data: bs58::decode(&compiled.data)
                    .into_vec()
                    .expect("Should base58 decode"),
                compute_info: None,
            },
            UiInstruction::Parsed(UiParsedInstruction::PartiallyDecoded(decoded)) => Self {
                program_id: Pubkey::from_str_const(&decoded.program_id),
                accounts: decoded
                    .accounts
                    .iter()
                    .map(|s| {
                        *parsed_accounts
                            .iter()
                            .find(|p| &p.signer.to_string() == s)
                            .expect("Should find pubkey")
                    })
                    .collect(),
                data: bs58::decode(&decoded.data)
                    .into_vec()
                    .expect("Should base58 decode"),
                compute_info: None,
            },
            // It's unclear how to parse already parsed ui transactions.
            UiInstruction::Parsed(UiParsedInstruction::Parsed(_)) => unimplemented!(),
        }
    }
}
