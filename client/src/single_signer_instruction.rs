use solana_instruction::Instruction;
use solana_sdk::signature::Keypair;

use crate::transactions::{
    CustomRpcClient,
    ParsedTransactionWithEvents,
};

/// A utility wrapper newtype for instructions that only need a single signer. This facilitates
/// simple construction and submission of single signer transactions with one instruction.
pub struct SingleSignerInstruction(Instruction);

impl SingleSignerInstruction {}

impl TryFrom<Instruction> for SingleSignerInstruction {
    type Error = anyhow::Error;

    fn try_from(instruction: Instruction) -> Result<Self, Self::Error> {
        if instruction.accounts.iter().fold(
            0,
            |acc, meta| {
                if meta.is_signer {
                    acc + 1
                } else {
                    acc
                }
            },
        ) != 1
        {
            return Err(anyhow::Error::msg(
                "This instruction requires more than one signer.",
            ));
        };
        Ok(Self(instruction))
    }
}

impl From<SingleSignerInstruction> for Instruction {
    fn from(instruction: SingleSignerInstruction) -> Self {
        instruction.0
    }
}

impl SingleSignerInstruction {
    pub async fn send_single_signer(
        self,
        rpc: &CustomRpcClient,
        signer: &Keypair,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        rpc.send_single_signer(signer, [self.0]).await
    }
}
