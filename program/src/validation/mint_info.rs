use dropset_interface::{
    error::DropsetError,
    state::market::MarketRef,
};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::pubkey_eq,
};
use pinocchio_token_interface::state::{
    load_unchecked as pinocchio_load_unchecked,
    mint::Mint,
};

#[derive(Clone)]
pub struct MintInfo<'a> {
    pub info: &'a AccountInfo,
    /// Flag for which mint this is. Facilitates skipping several pubkey comparisons.
    pub is_base_mint: bool,
}

impl<'a> MintInfo<'a> {
    #[inline(always)]
    pub fn new(info: &'a AccountInfo, market: MarketRef) -> Result<MintInfo<'a>, ProgramError> {
        if pubkey_eq(info.key(), &market.header.base_mint) {
            Ok(Self {
                info,
                is_base_mint: true,
            })
        } else if pubkey_eq(info.key(), &market.header.quote_mint) {
            Ok(Self {
                info,
                is_base_mint: false,
            })
        } else {
            Err(DropsetError::InvalidMintAccount.into())
        }
    }

    /// Verifies the `base` and `quote` account info passed in is valid according to the pubkeys
    /// stored in the market header.
    #[inline(always)]
    pub fn new_base_and_quote(
        base: &'a AccountInfo,
        quote: &'a AccountInfo,
        market: MarketRef,
    ) -> Result<(MintInfo<'a>, MintInfo<'a>), DropsetError> {
        // The two mints in the header will never be invalid since they're checked prior to
        // initialization and never updated, so the only thing that's necessary to check is that the
        // account info pubkeys match the ones in the header.
        if !pubkey_eq(base.key(), &market.header.base_mint)
            || !pubkey_eq(quote.key(), &market.header.quote_mint)
        {
            return Err(DropsetError::InvalidMintAccount);
        }

        Ok((
            Self {
                info: base,
                is_base_mint: true,
            },
            Self {
                info: quote,
                is_base_mint: false,
            },
        ))
    }

    /// Borrows the mint account's data to get the mint decimals.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - WRITE accounts are not currently borrowed in *any* capacity.
    /// - READ accounts are not currently mutably borrowed.
    ///
    /// ### Accounts
    ///   0. `[READ]` Mint account
    #[inline(always)]
    pub unsafe fn get_mint_decimals(&self) -> Result<u8, ProgramError> {
        let data = unsafe { self.info.borrow_data_unchecked() };
        // Safety: `MintInfo` is verified in the market header and thus can only be constructed if a
        // mint account is initialized.
        Ok(unsafe { pinocchio_load_unchecked::<Mint>(data) }?.decimals)
    }
}
