use dropset_interface::error::DropsetError;
use pinocchio::{account_info::AccountInfo, pubkey::pubkey_eq};

#[derive(Clone)]
pub struct TokenProgramInfo<'a> {
    pub info: &'a AccountInfo,
    /// To avoid pubkey comparisons later, store whether or not this is the base or 2022 program.
    pub is_spl_token: bool,
}

impl<'a> TokenProgramInfo<'a> {
    #[inline(always)]
    pub fn new(info: &'a AccountInfo) -> Result<TokenProgramInfo<'a>, DropsetError> {
        let is_spl_token = pubkey_eq(info.key(), &pinocchio_token::ID);

        if !is_spl_token && !pubkey_eq(info.key(), &pinocchio_token_2022::ID) {
            return Err(DropsetError::InvalidTokenProgram);
        }

        Ok(Self { info, is_spl_token })
    }
}
