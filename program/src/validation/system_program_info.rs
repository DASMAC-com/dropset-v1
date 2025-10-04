use dropset_interface::{error::DropsetError, state::SYSTEM_PROGRAM_ID};
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

#[derive(Clone)]
pub struct SystemProgramInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> SystemProgramInfo<'a> {
    #[inline(always)]
    pub fn new(info: &'a AccountInfo) -> Result<SystemProgramInfo<'a>, DropsetError> {
        if !pubkey_eq(info.key(), &SYSTEM_PROGRAM_ID) {
            return Err(DropsetError::IncorrectSystemProgram);
        }
        Ok(SystemProgramInfo { info })
    }

    #[inline(always)]
    pub fn new_unchecked(info: &'a AccountInfo) -> SystemProgramInfo<'a> {
        SystemProgramInfo { info }
    }
}
