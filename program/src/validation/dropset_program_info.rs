use dropset_interface::error::DropsetError;
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

#[derive(Clone)]
pub struct DropsetProgramInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> DropsetProgramInfo<'a> {
    #[inline(always)]
    pub fn new(info: &'a AccountInfo) -> Result<DropsetProgramInfo<'a>, DropsetError> {
        if !pubkey_eq(info.key(), &crate::ID) {
            return Err(DropsetError::IncorrectDropsetProgram);
        }
        Ok(Self { info })
    }
}
