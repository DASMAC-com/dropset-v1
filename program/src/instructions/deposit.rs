use pinocchio::{account_info::AccountInfo, ProgramResult};

pub fn process_deposit(_accounts: &[AccountInfo], _instruction_data: &[u8]) -> ProgramResult {
    Ok(())
}
