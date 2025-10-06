use pinocchio::{account_info::AccountInfo, ProgramResult};

pub fn process_flush_events(_accounts: &[AccountInfo], _instruction_data: &[u8]) -> ProgramResult {
    Ok(())
}
