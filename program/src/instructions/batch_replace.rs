//! See [`process_batch_replace`].

use dropset_interface::instructions::BatchReplaceInstructionData;
use pinocchio::{
    account::AccountView,
    ProgramResult,
};

use crate::context::mutate_orders_context::MutateOrdersContext;

/// Handler logic for batching multiple cancel + place order instructions in a single atomic
/// instruction.
///
/// # Safety
///
/// Since the accounts borrowed depend on the inner batch instructions, the most straightforward
/// safety contract is simply ensuring that **no Solana account data is currently borrowed** prior
/// to calling this instruction.
#[inline(never)]
pub unsafe fn process_batch_replace(
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let BatchReplaceInstructionData {
        user_sector_index_hint,
        new_bids,
        new_asks,
    } = BatchReplaceInstructionData::unpack_untagged(instruction_data)?;

    let num_new_bids = new_bids.num_orders() as usize;
    let num_new_asks = new_asks.num_orders() as usize;

    // Safety: No account data in `accounts` is currently borrowed.
    let mut ctx = unsafe { MutateOrdersContext::load(accounts)? };

    Ok(())
}
