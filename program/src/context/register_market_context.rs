use dropset_interface::error::DropsetError;
use pinocchio::account_info::AccountInfo;

use crate::validation::uninitialized_account_info::UninitializedAccountInfo;

#[derive(Clone)]
pub struct RegisterMarketContext<'a> {
    pub user: &'a AccountInfo,
    pub market_account: UninitializedAccountInfo<'a>,
    pub base_market_ata: &'a AccountInfo,
    pub quote_market_ata: &'a AccountInfo,
    pub base_mint: &'a AccountInfo,
    pub quote_mint: &'a AccountInfo,
    pub base_token_program: &'a AccountInfo,
    pub quote_token_program: &'a AccountInfo,
    pub _ata_program: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
}

impl<'a> RegisterMarketContext<'a> {
    pub fn load(accounts: &'a [AccountInfo]) -> Result<RegisterMarketContext<'a>, DropsetError> {
        #[rustfmt::skip]
        let [
            user,
            market_account,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            _ata_program,
            system_program,
        ] = accounts else {
            return Err(DropsetError::IncorrectNumberOfAccountInfos);
        };

        // Since the market PDA and both of its associated token accounts are created atomically
        // during market registration, all derivations are guaranteed to be correct if the
        // transaction succeeds. The two mint accounts are also guaranteed to be different, since
        // the non-idempotent ATA creation instruction would fail on the second invocation.
        // Thus there is no need to check ownership, address derivations, or account data here, only
        // that the `market_account` is uninitialized.
        // The token programs are also validated in the ATA `Create` instruction.
        // The associated token program is hard-coded as an address and never even used.
        let market_account = UninitializedAccountInfo::new(market_account)?;

        Ok(Self {
            user,
            market_account,
            base_market_ata,
            quote_market_ata,
            base_mint,
            quote_mint,
            base_token_program,
            quote_token_program,
            _ata_program,
            system_program,
        })
    }
}
