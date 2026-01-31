//! See [`process_register_market`].

use dropset_interface::{
    error::DropsetError,
    events::RegisterMarketEventInstructionData,
    instructions::RegisterMarketInstructionData,
    state::{
        market_header::MarketHeader,
        sector::SECTOR_SIZE,
        transmutable::Transmutable,
    },
};
use pinocchio::{
    account::AccountView,
    error::ProgramError,
    sysvars::{
        rent::Rent,
        Sysvar,
    },
    Address,
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
    validation::market_account_view::MarketAccountView,
};

/// Instruction handler logic for initializing a new market account and its header metadata.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in
/// [`dropset_interface::instructions::generated_program::RegisterMarket`].
#[inline(never)]
pub unsafe fn process_register_market<'a>(
    accounts: &'a [AccountView],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let num_sectors = RegisterMarketInstructionData::unpack_untagged(instruction_data)?.num_sectors;
    let ctx = RegisterMarketContext::load(accounts)?;

    // It's not necessary to check the returned PDA here because `CreateAccount` will fail if the
    // market account info's address doesn't match.
    let (_pda, market_bump) = Address::try_find_program_address(
        market_seeds!(ctx.base_mint.address(), ctx.quote_mint.address()),
        &crate::ID,
    )
    .ok_or(DropsetError::AddressDerivationFailed)?;

    // Calculate the lamports required to create the market account.
    let account_space = MarketHeader::LEN + SECTOR_SIZE * (num_sectors as usize);
    let lamports_required = Rent::get()?.try_minimum_balance(account_space)?;

    // Create the market account PDA.
    pinocchio_system::instructions::CreateAccount {
        from: ctx.user,                 // WRITE
        to: ctx.market_account.account, // WRITE
        lamports: lamports_required,
        space: account_space as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[market_signer!(
        ctx.base_mint.address(),
        ctx.quote_mint.address(),
        market_bump
    )])?;

    // Create the market's base and quote mint associated token accounts with the non-idempotent
    // creation instruction to ensure that passing duplicate mint accounts fails.
    pinocchio_associated_token_account::instructions::Create {
        funding_account: ctx.user,             // WRITE
        account: ctx.base_market_ata,          // WRITE
        wallet: ctx.market_account.account,    // READ
        mint: ctx.base_mint,                   // READ
        system_program: ctx.system_program,    // READ
        token_program: ctx.base_token_program, // READ
    }
    .invoke()?;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: ctx.user,              // WRITE
        account: ctx.quote_market_ata,          // WRITE
        wallet: ctx.market_account.account,     // READ
        mint: ctx.quote_mint,                   // READ
        system_program: ctx.system_program,     // READ
        token_program: ctx.quote_token_program, // READ
    }
    .invoke()?;

    initialize_market_account_data(
        // Safety: Scoped mutable borrow of the market account data to initialize it.
        unsafe { ctx.market_account.account.borrow_unchecked_mut() },
        ctx.base_mint.address(),
        ctx.quote_mint.address(),
        market_bump,
    )?;

    // Safety: `ctx.market_account.account` was just initialized as a market account.
    let market_account = unsafe { MarketAccountView::new_unchecked(ctx.market_account.account) };

    event_buffer.add_to_buffer(
        RegisterMarketEventInstructionData::new(*market_account.account().address()),
        ctx.event_authority,
        market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account,
    })
}
