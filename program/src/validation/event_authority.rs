use dropset_interface::{
    error::DropsetError,
    seeds::event_authority,
};
use pinocchio::account_info::AccountInfo;

#[derive(Clone)]
pub struct EventAuthorityInfo<'a> {
    pub _info: &'a AccountInfo,
}

impl<'a> EventAuthorityInfo<'a> {
    #[inline(always)]
    pub fn new(event_authority_account: &'a AccountInfo) -> Result<Self, DropsetError> {
        if event_authority_account.key() != &event_authority::ID {
            return Err(DropsetError::IncorrectEventAuthority);
        }

        if !event_authority_account.is_signer() {
            return Err(DropsetError::EventAuthorityMustBeSigner);
        }

        Ok(Self {
            _info: event_authority_account,
        })
    }
}
