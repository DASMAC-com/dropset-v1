use dropset_interface::{
    instructions::num_sectors::NumSectorsInstructionData,
    state::{
        market::Market, market_header::MARKET_HEADER_SIZE, sector::SECTOR_SIZE, transmutable::load,
    },
};
use pinocchio::{
    account_info::AccountInfo,
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
    // Safety: All bit patterns are valid.
    let num_sectors = unsafe { load::<NumSectorsInstructionData>(instruction_data) }?.num_sectors();

    let ctx = RegisterMarketContext::load(accounts)?;

    // Prepare the PDA signer seeds.
    let (base_mint, quote_mint, market_bump) = {
        // Safety: Single immutable borrow to the market account data.
        let data = unsafe { ctx.market_account.info.borrow_data_unchecked() };
        let market = Market::from_bytes_unchecked(data)?;
        let header = market.header;
        (header.base_mint, header.quote_mint, header.market_bump)
    };

    // Create the program derived market account.
    let account_space = MARKET_HEADER_SIZE + SECTOR_SIZE * (num_sectors as usize);
    let lamports_required = Rent::get()?.minimum_balance(account_space);

    pinocchio_system::instructions::CreateAccount {
        from: ctx.user,
        to: ctx.market_account.info,
        lamports: lamports_required,
        space: account_space as u64,
        owner: &crate::ID,
    }
    .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])?;

    // Create the market's base and quote associated token accounts.
    create_token_accounts::create_atas(&ctx)?;

    initialize_market_account_data(
        // Safety: Single mutable borrow of the market account data for the init call.
        unsafe { ctx.market_account.info.borrow_mut_data_unchecked() },
        &base_mint,
        &quote_mint,
        market_bump,
    )?;

    Ok(())
}
