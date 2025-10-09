use dropset_interface::{
    error::DropsetError,
    instructions::num_sectors::NumSectorsInstructionData,
    state::{market_header::MarketHeader, sector::SECTOR_SIZE, transmutable::Transmutable},
};
use pinocchio::{
    account_info::AccountInfo,
    pubkey::try_find_program_address,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

use crate::{
    context::register_market_context::RegisterMarketContext,
    market_signer,
    shared::{
        market_operations::initialize_market_account_data,
        token_utils::create_token_accounts::{self},
    },
};

pub fn process_register_market(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let num_sectors = NumSectorsInstructionData::load(instruction_data)?.num_sectors();
    let ctx = RegisterMarketContext::load(accounts)?;

    // It's not necessary to check the returned PDA here because `CreateAccount` will fail if the
    // market account info's pubkey doesn't match.
    let (_pda, market_bump) =
        try_find_program_address(&[ctx.base_mint.key(), ctx.quote_mint.key()], &crate::ID)
            .ok_or(DropsetError::AddressDerivationFailed)?;

    // Create the program derived market account.
    let account_space = MarketHeader::LEN + SECTOR_SIZE * (num_sectors as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);
    pinocchio_system::instructions::CreateAccount {
        from: ctx.user,
        to: ctx.market_account.info,
        lamports: lamports_required,
        space: account_space as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[market_signer!(
        ctx.base_mint.key(),
        ctx.quote_mint.key(),
        market_bump
    )])?;

    // Create the market's base and quote associated token accounts.
    create_token_accounts::create_atas(&ctx)?;

    initialize_market_account_data(
        // Safety: Single mutable borrow of the market account data for the init call.
        unsafe { ctx.market_account.info.borrow_mut_data_unchecked() },
        ctx.base_mint.key(),
        ctx.quote_mint.key(),
        market_bump,
    )?;

    Ok(())
}
