use pinocchio::{
    account_info::AccountInfo,
    pubkey::{
        pubkey_eq,
        Pubkey,
    },
};

#[inline(always)]
pub fn owned_by(info: &AccountInfo, potential_owner: &Pubkey) -> bool {
    pubkey_eq(info.owner(), potential_owner)
}

/// Checks if an account is owned by the `spl_token::ID`; i.e., not `spl_token_2022::ID`.
///
/// Note that this in and of itself isn't sufficient proof of a valid, initialized mint account.
/// You must either check that the account's data length is > 0 or indirectly validate it by calling
/// the program with the mint account.
#[inline(always)]
pub fn is_owned_by_spl_token(info: &AccountInfo) -> bool {
    owned_by(info, &pinocchio_token::ID)
}
