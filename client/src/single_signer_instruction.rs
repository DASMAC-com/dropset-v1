use solana_instruction::Instruction;
use solana_sdk::signature::Keypair;

use crate::transactions::{
    CustomRpcClient,
    ParsedTransactionWithEvents,
};

/// A utility wrapper newtype for instructions that only need a single signer. This facilitates
/// simple construction and submission of single signer transactions with one instruction.
pub struct SingleSignerInstruction(Instruction);

impl TryFrom<Instruction> for SingleSignerInstruction {
    type Error = anyhow::Error;

    fn try_from(instruction: Instruction) -> Result<Self, Self::Error> {
        let num_signers = instruction
            .accounts
            .iter()
            .filter(|meta| meta.is_signer)
            .count();
        if num_signers != 1 {
            return Err(anyhow::anyhow!(
                "Expected a single signer instruction, got {num_signers} signers."
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
