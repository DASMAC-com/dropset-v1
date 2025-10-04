use dropset_interface::{error::DropsetError, program, utils::owned_by};
use pinocchio::account_info::AccountInfo;

#[derive(Clone)]
pub struct MarketAccountInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> MarketAccountInfo<'a> {
    #[inline(always)]
    /// Only checks that the account is owned by this program to allow the caller to validate
    /// internal program data on a case by case basis.
    pub fn new(info: &'a AccountInfo) -> Result<MarketAccountInfo<'a>, DropsetError> {
        if !owned_by(info, &program::ID) {
            return Err(DropsetError::InvalidMarketAccountOwner);
        }

        Ok(Self { info })
    }
}
