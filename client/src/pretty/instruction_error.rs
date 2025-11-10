//! Interprets RPC and on-chain errors into readable `dropset`/Solana instruction error messages.

use std::fmt::Display;

use dropset_interface::{
    error::DropsetError,
    instructions::DropsetInstruction,
};
use solana_client::{
    client_error::{
        ClientError,
        ClientErrorKind,
    },
    rpc_request::{
        RpcError::RpcResponseError,
        RpcResponseErrorData,
    },
    rpc_response::RpcSimulateTransactionResult,
};
use solana_instruction::Instruction;
use solana_instruction_error::InstructionError as SolanaInstructionError;
use solana_transaction_error::TransactionError;

use crate::{
    fmt_kv,
    LogColor,
};

enum InstructionError {
    Solana {
        instruction_tag: u8,
        error: SolanaInstructionError,
    },
    Dropset {
        dropset_instruction: DropsetInstruction,
        error: DropsetError,
    },
}

pub struct PrettyInstructionError(InstructionError);

impl PrettyInstructionError {
    pub fn new(error: &ClientError, instructions: &[Instruction]) -> Option<Self> {
        match error.kind() {
            ClientErrorKind::RpcError(RpcResponseError {
                data:
                    RpcResponseErrorData::SendTransactionPreflightFailure(
                        RpcSimulateTransactionResult {
                            err: Some(ui_err), ..
                        },
                    ),
                ..
            }) => {
                let transaction_error: TransactionError = ui_err.clone().into();
                match transaction_error {
                    TransactionError::InstructionError(instruction_index, instruction_error) => {
                        let instruction = instructions
                            .get(instruction_index as usize)
                            .expect("Instruction index from error should be valid");
                        let instruction_tag = instruction.data[0];

                        let res = match instruction_error {
                            SolanaInstructionError::Custom(code) => {
                                if instruction.program_id.as_ref() == dropset::ID {
                                    let dropset_error = DropsetError::from_repr(code as u8)
                                        .expect("Should be valid");
                                    let dropset_tag = DropsetInstruction::try_from(instruction_tag)
                                        .expect("Should be valid");

                                    Self(InstructionError::Dropset {
                                        dropset_instruction: dropset_tag,
                                        error: dropset_error,
                                    })
                                } else {
                                    Self(InstructionError::Solana {
                                        instruction_tag,
                                        error: SolanaInstructionError::Custom(code),
                                    })
                                }
                            }
                            instruction_error => Self(InstructionError::Solana {
                                instruction_tag,
                                error: instruction_error,
                            }),
                        };

                        Some(res)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Display for PrettyInstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (error_type, instruction, error) = match &self.0 {
            InstructionError::Solana {
                instruction_tag,
                error,
            } => (
                "SolanaInstructionError",
                instruction_tag.to_string(),
                error.to_string(),
            ),
            InstructionError::Dropset {
                dropset_instruction,
                error,
            } => (
                "DropsetError",
                dropset_instruction.to_string(),
                error.to_string(),
            ),
        };

        let message = format!("{instruction}, {error})");
        let error_message = fmt_kv!(error_type, message, LogColor::Error);
        writeln!(f, "{error_message}")
    }
}
