//! CU benchmark: Pack/Unpack vs Borsh deserialization of
//! [`dropset_interface::instructions::BatchReplaceInstructionData`].

#![no_std]

use core::hint::black_box;

use pinocchio::{
    account::AccountView,
    error::ProgramError,
    no_allocator,
    nostd_panic_handler,
    program_entrypoint,
    Address,
    ProgramResult,
};

program_entrypoint!(process_instruction);
no_allocator!();
nostd_panic_handler!();

#[inline(never)]
fn process_instruction(
    _program_id: &Address,
    _accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    // The pack version.
    #[cfg(feature = "bench-program-A")]
    {
        use dropset_interface::{
            instructions::BatchReplaceInstructionData,
            state::user_order_sectors::MAX_ORDERS_USIZE,
        };
        use price::OrderInfoArgs;

        let data = BatchReplaceInstructionData::unpack_untagged(instruction_data)?;

        if data.user_sector_index_hint == u32::MAX {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Cast UnvalidatedOrders to [OrderInfoArgs; MAX_ORDERS_USIZE] to get
        // access to the underlying arrays without having to update the public API.
        let bids_ptr = &data.new_bids as *const _ as *const [OrderInfoArgs; MAX_ORDERS_USIZE];
        let new_bids = unsafe { &*bids_ptr };
        let asks_ptr = &data.new_asks as *const _ as *const [OrderInfoArgs; MAX_ORDERS_USIZE];
        let new_asks = unsafe { &*asks_ptr };

        // Use black_box to prevent the compiler from optimizing away field accesses.
        black_box(new_bids);
        black_box(new_asks);
    }

    // The borsh version.
    #[cfg(feature = "bench-program-B")]
    {
        use borsh::BorshDeserialize;

        #[derive(BorshDeserialize)]
        struct BorshOrderInfoArgs {
            price_mantissa: u32,
            base_scalar: u64,
            base_exponent_biased: u8,
            quote_exponent_biased: u8,
        }

        #[derive(BorshDeserialize)]
        struct BorshUnvalidatedOrders {
            order_args: [BorshOrderInfoArgs; 5],
        }

        #[derive(BorshDeserialize)]
        struct BorshBatchReplaceData {
            user_sector_index_hint: u32,
            new_bids: BorshUnvalidatedOrders,
            new_asks: BorshUnvalidatedOrders,
        }

        let data = BorshBatchReplaceData::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        if data.user_sector_index_hint == u32::MAX {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Use black_box to prevent the compiler from optimizing away field accesses.
        black_box(&data.new_bids.order_args);
        black_box(&data.new_asks.order_args);
    }

    Ok(())
}
