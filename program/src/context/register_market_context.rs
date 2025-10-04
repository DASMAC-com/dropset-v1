use dropset_interface::error::DropsetError;
use pinocchio::account_info::AccountInfo;

use crate::validation::{
    system_program_info::SystemProgramInfo, token_program_info::TokenProgramInfo,
    uninitialized_account_info::UninitializedAccountInfo,
};

#[derive(Clone)]
pub struct RegisterMarketContext<'a> {
    pub registrant: &'a AccountInfo,
    pub market_account: UninitializedAccountInfo<'a>,
    pub base_mint: &'a AccountInfo,
    pub quote_mint: &'a AccountInfo,
    pub vault_base_ata: UninitializedAccountInfo<'a>,
    pub vault_quote_ata: UninitializedAccountInfo<'a>,
    pub base_token_program: TokenProgramInfo<'a>,
    pub quote_token_program: TokenProgramInfo<'a>,
    pub system_program: SystemProgramInfo<'a>,
}

impl<'a> RegisterMarketContext<'a> {
    pub fn load(accounts: &'a [AccountInfo]) -> Result<RegisterMarketContext<'a>, DropsetError> {
        let [registrant, market_account, base_mint, quote_mint, vault_base_ata, vault_quote_ata, base_token_program, quote_token_program, system_program] =
            accounts
        else {
            return Err(DropsetError::NotEnoughAccountKeys);
        };
        // Since the market PDA and both of its associated token accounts are created atomically
        // during market registration, all derivations are guaranteed to be correct if the
        // transaction succeeds. The two mint accounts are also guaranteed to be different, since
        // the non-idempotent ATA creation instruction would fail on the second invocation.
        // Thus there is no need to check ownership, address derivations, or account data here.
        let market_account = UninitializedAccountInfo::new(market_account)?;
        let vault_base_ata = UninitializedAccountInfo::new_unchecked(vault_base_ata);
        let vault_quote_ata = UninitializedAccountInfo::new_unchecked(vault_quote_ata);
        // Also unchecked because the system program is checked and used in the token programs.
        let system_program = SystemProgramInfo::new_unchecked(system_program);

        // These checks are necessary to know which program to call later.
        let base_token_program = TokenProgramInfo::new(base_token_program)?;
        let quote_token_program = TokenProgramInfo::new(quote_token_program)?;

        Ok(Self {
            registrant,
            market_account,
            base_mint,
            quote_mint,
            vault_base_ata,
            vault_quote_ata,
            base_token_program,
            quote_token_program,
            system_program,
        })
    }
}
