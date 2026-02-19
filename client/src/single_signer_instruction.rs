use std::future::Future;

use solana_instruction::Instruction;
use solana_sdk::signature::Keypair;

use crate::transactions::{
    CustomRpcClient,
    ParsedTransactionWithEvents,
};

/// Extension trait for submitting an instruction as a single-signer transaction.
///
/// Import this trait where you need `.send_single_signer()` on a built [`Instruction`].
/// The signer count check is performed at submission time.
pub trait SingleSignerInstruction {
    fn send_single_signer(
        self,
        rpc: &CustomRpcClient,
        signer: &Keypair,
    ) -> impl Future<Output = anyhow::Result<ParsedTransactionWithEvents>> + Send;
}

impl SingleSignerInstruction for Instruction {
    async fn send_single_signer(
        self,
        rpc: &CustomRpcClient,
        signer: &Keypair,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let num_signers = self.accounts.iter().filter(|meta| meta.is_signer).count();
        if num_signers != 1 {
            return Err(anyhow::anyhow!(
                "Expected a single signer instruction, got {num_signers} signers."
            ));
        }
        rpc.send_single_signer(signer, [self]).await
    }
}
