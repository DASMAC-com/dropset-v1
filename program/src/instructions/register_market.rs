use dropset_interface::{
    instructions::num_sectors::NumSectorsInstructionData, state::transmutable::load,
};
use pinocchio::{account_info::AccountInfo, ProgramResult};

pub fn process_register_market(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    // Safety: All bit patterns are valid.
    let num_sectors = unsafe { load::<NumSectorsInstructionData>(instruction_data) }?.num_sectors();

    Ok(())
}
