use dropset_interface::{
    error::DropsetError,
    state::SYSTEM_PROGRAM_ID,
    utils::owned_by,
};
use pinocchio::account_info::AccountInfo;

/// Represents a completely uninitialized account.
#[derive(Clone)]
pub struct UninitializedAccountInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> UninitializedAccountInfo<'a> {
    #[inline(always)]
    pub fn new(info: &'a AccountInfo) -> Result<UninitializedAccountInfo<'a>, DropsetError> {
        if !info.data_is_empty() {
            return Err(DropsetError::AlreadyInitializedAccount);
        }

        if !owned_by(info, &SYSTEM_PROGRAM_ID) {
            return Err(DropsetError::NotOwnedBySystemProgram);
        }

        Ok(Self { info })
    }
}
