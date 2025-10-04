use pinocchio::{
    account_info::AccountInfo,
    pubkey::{pubkey_eq, Pubkey},
};

#[inline(always)]
pub fn owned_by(info: &AccountInfo, expected_owner: &Pubkey) -> bool {
    pubkey_eq(info.owner(), expected_owner)
}
