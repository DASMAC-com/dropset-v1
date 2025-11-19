//! See [`process_register_market`].

use dropset_interface::{
    error::DropsetError,
    events::RegisterMarketEventInstructionData,
    instructions::generated_pinocchio::*,
    state::{
        market_header::MarketHeader,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    },
};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::try_find_program_address,
    sysvars::{
        rent::Rent,
        Sysvar,
    },
};

use crate::{
    context::{
        register_market_context::RegisterMarketContext,
        EventBufferContext,
    },
    events::EventBuffer,
    market_seeds,
    market_signer,
    shared::market_operations::initialize_market_account_data,
    validation::market_account_info::MarketAccountInfo,
};

/// Instruction handler logic for initializing a new market account and its header metadata.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in [`RegisterMarket`].
#[inline(never)]
pub unsafe fn process_register_market<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
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

    // Safety: `ctx.market_account.info` was just initialized as a market account.
    let market_account = unsafe { MarketAccountInfo::new_unchecked(ctx.market_account.info) };

    event_buffer.add_to_buffer(
        RegisterMarketEventInstructionData::new(*market_account.info().key()),
        ctx.event_authority,
        market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account,
    })
}
