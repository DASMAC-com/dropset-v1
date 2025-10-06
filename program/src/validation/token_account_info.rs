use dropset_interface::{error::DropsetError, utils::owned_by};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::{pubkey_eq, Pubkey},
};
use pinocchio_token_interface::state::{
    account::Account, load as pinocchio_load, load_unchecked as pinocchio_load_unchecked,
};

#[derive(Clone)]
pub struct TokenAccountInfo<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> TokenAccountInfo<'a> {
    #[inline(always)]
    pub fn new(
        info: &'a AccountInfo,
        expected_mint: &Pubkey,
        expected_owner: &Pubkey,
    ) -> Result<TokenAccountInfo<'a>, ProgramError> {
        // NOTE: This check is most likely unnecessary since the token program checks this and fails
        // transfers if the check fails.
        if !owned_by(info, &pinocchio_token::ID) && !owned_by(info, &pinocchio_token_2022::ID) {
            return Err(DropsetError::OwnerNotTokenProgram.into());
        }

        let account_data = &info.try_borrow_data()?;

        // Note the load below also checks that the account has been initialized.
        // Safety: Mint info account owner has been verified, so the account data is valid.
        let mint_token_account = unsafe { pinocchio_load::<Account>(account_data) }?;

        if !pubkey_eq(&mint_token_account.mint, expected_mint) {
            return Err(DropsetError::MintInfoMismatch.into());
        }
        if !pubkey_eq(&mint_token_account.owner, expected_owner) {
            return Err(DropsetError::IncorrectTokenAccountOwner.into());
        }

        Ok(Self { info })
    }

    #[inline(always)]
    pub fn get_balance(&self) -> Result<u64, ProgramError> {
        let data = &self.info.try_borrow_data()?;
        // Safety: Account is verified as initialized and owned by one of the spl token programs
        // upon construction of Self.
        Ok(unsafe { pinocchio_load_unchecked::<Account>(data) }?.amount())
    }
}
