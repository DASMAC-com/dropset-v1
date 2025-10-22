use dropset_interface::{
    error::DropsetError,
    instructions::generated_pinocchio::RegisterMarketInstructionData,
    state::{
        market_header::MarketHeader,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    },
};
use pinocchio::{
    account_info::AccountInfo,
    pubkey::try_find_program_address,
    sysvars::{
        rent::Rent,
        Sysvar,
    },
    ProgramResult,
};

use crate::{
    context::register_market_context::RegisterMarketContext,
    market_seeds,
    market_signer,
    shared::market_operations::initialize_market_account_data,
};

/// # Safety
///
/// Caller guarantees the safety contract detailed in
/// [`dropset_interface::instructions::register_market::RegisterMarket`]
pub unsafe fn process_register_market(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let num_sectors = RegisterMarketInstructionData::unpack(instruction_data)?.num_sectors;
    let ctx = RegisterMarketContext::load(accounts)?;

    // It's not necessary to check the returned PDA here because `CreateAccount` will fail if the
    // market account info's pubkey doesn't match.
    let (_pda, market_bump) = try_find_program_address(
        market_seeds!(ctx.base_mint.key(), ctx.quote_mint.key()),
        &crate::ID,
    )
    .ok_or(DropsetError::AddressDerivationFailed)?;

    // Calculate the lamports required to create the market account.
    let account_space = MarketHeader::LEN + SECTOR_SIZE * (num_sectors as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);

    // Create the market account PDA.
    pinocchio_system::instructions::CreateAccount {
        from: ctx.user,              // WRITE
        to: ctx.market_account.info, // WRITE
        lamports: lamports_required,
        space: account_space as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[market_signer!(
        ctx.base_mint.key(),
        ctx.quote_mint.key(),
        market_bump
    )])?;

    // Create the market's base and quote mint associated token accounts with the non-idempotent
    // creation instruction to ensure that passing duplicate mint accounts fails.
    pinocchio_associated_token_account::instructions::Create {
        funding_account: ctx.user,             // WRITE
        account: ctx.base_market_ata,          // WRITE
        wallet: ctx.market_account.info,       // READ
        mint: ctx.base_mint,                   // READ
        system_program: ctx.system_program,    // READ
        token_program: ctx.base_token_program, // READ
    }
    .invoke()?;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: ctx.user,              // WRITE
        account: ctx.quote_market_ata,          // WRITE
        wallet: ctx.market_account.info,        // READ
        mint: ctx.quote_mint,                   // READ
        system_program: ctx.system_program,     // READ
        token_program: ctx.quote_token_program, // READ
    }
    .invoke()?;

    initialize_market_account_data(
        // Safety: Scoped mutable borrow of the market account data to initialize it.
        unsafe { ctx.market_account.info.borrow_mut_data_unchecked() },
        ctx.base_mint.key(),
        ctx.quote_mint.key(),
        market_bump,
    )?;

    Ok(())
}
