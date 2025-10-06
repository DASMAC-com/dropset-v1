use crate::validation::market_account_info::MarketAccountInfo;
use dropset_interface::{error::DropsetError, state::market::MarketRef};
use pinocchio::{account_info::AccountInfo, program_error::ProgramError, pubkey::pubkey_eq};
use pinocchio_token_interface::state::{load_unchecked as pinocchio_load_unchecked, mint::Mint};

#[derive(Clone)]
pub struct MintInfo<'a> {
    pub info: &'a AccountInfo,
    /// Flag for which mint this is. Facilitates skipping several pubkey comparisons.
    pub is_base_mint: bool,
}

impl<'a> MintInfo<'a> {
    #[inline(always)]
    /// Checks that the account matches either the base or quote mint in the market header and
    /// records which one it is.
    pub fn new(
        info: &'a AccountInfo,
        market_account: &MarketAccountInfo,
    ) -> Result<MintInfo<'a>, ProgramError> {
        let data = &market_account.info.try_borrow_data()?;
        let market = MarketRef::from_bytes(data)?;

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

    #[inline(always)]
    /// Verifies the `base` and `quote` account info passed in is valid according to the pubkeys
    /// stored in the market header.
    ///
    /// # Safety
    ///
    /// Caller guarantees there are no active borrows on the market account data.
    pub unsafe fn new_base_and_quote(
        base: &'a AccountInfo,
        quote: &'a AccountInfo,
        market_account: &MarketAccountInfo,
    ) -> Result<(MintInfo<'a>, MintInfo<'a>), DropsetError> {
        // Safety: Caller guarantees exclusive access to the market data.
        let market = market_account.load_unchecked()?;

        // The two mints will never be invalid since they're checked prior to initialization and
        // never updated, so the only thing that's necessary to check is that the account info
        // pubkeys match the ones in the header.
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

    /// Safely borrows the mint account's data to get the mint decimals.
    pub fn get_mint_decimals(&self) -> Result<u8, ProgramError> {
        let data = &self.info.try_borrow_data()?;
        // Safety: `MintInfo` is verified in the market header and thus can only be constructed if a
        // mint account is initialized.
        Ok(unsafe { pinocchio_load_unchecked::<Mint>(data) }?.decimals)
    }
}
