use dropset_interface::{
    instructions::close::CloseInstructionData,
    state::{node::Node, transmutable::Transmutable},
};
use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::{
    context::close_context::CloseContext, market_signer,
    shared::market_operations::find_seat_with_hint,
};

/// Closes a market seat for a user by withdrawing all base and quote from their seat.
///
/// # Safety
///
/// Caller guarantees:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Accounts
///   0. `[WRITE]` Market account
///   1. `[WRITE]` Market base mint token account
///   2. `[WRITE]` Market quote mint token account
///   3. `[WRITE]` User base mint token account
///   4. `[WRITE]` User quote mint token account
///   5. `[READ]` Base mint
///   6. `[READ]` Quote mint
pub fn process_close(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let mut ctx = unsafe { CloseContext::load(accounts) }?;

    // Safety: All bit patterns are valid.
    let args = CloseInstructionData::load(instruction_data)?;
    let hint = args.sector_index_hint();

    // Get the market bump and the base and quote amounts available for the user.
    let (market_bump, base_available, quote_available) = unsafe {
        // Safety: Scoped immutable borrow of market account data.
        let market = ctx.market_account.load_unchecked();
        let market_bump = market.header.market_bump;

        Node::check_in_bounds(market.sectors, hint)?;
        // Safety: The index hint was just verified as in-bounds.
        let seat = find_seat_with_hint(market, hint, ctx.user.key())?;

        // NOTE: The base/quote available and deposited do not need to be zeroed here because they're
        // zeroed out in the `push_free_node` call in the `remove_at` method below.
        (market_bump, seat.base_available(), seat.quote_available())
    };

    // Remove the seat, push it to the free stack, and zero it out.
    unsafe {
        ctx.market_account
            // Safety: Scoped mutable borrow of market account data to remove the seat.
            .load_unchecked_mut()
            .seat_list()
            // Safety: The index hint was verified as in-bounds.
            .remove_at(hint)
    };

    // If the user had any `base_available`, transfer that amount from market account => user.
    if base_available > 0 {
        if ctx.base_token_program.is_spl_token {
            pinocchio_token::instructions::Transfer {
                from: ctx.market_base_ata.info,       // WRITE
                to: ctx.user_base_ata.info,           // WRITE
                authority: ctx.market_account.info(), // READ
                amount: base_available,
            }
            .invoke_signed(&[market_signer!(
                ctx.base_mint.info.key(),
                ctx.quote_mint.info.key(),
                market_bump
            )])?;
        } else {
            // Safety: Scoped immutable borrow of mint account data to get mint decimals.
            let decimals = unsafe { ctx.base_mint.get_mint_decimals() }?;
            pinocchio_token::instructions::TransferChecked {
                from: ctx.market_base_ata.info,       // WRITE
                to: ctx.user_base_ata.info,           // WRITE
                authority: ctx.market_account.info(), // READ
                mint: ctx.base_mint.info,             // READ
                amount: base_available,
                decimals,
            }
            .invoke_signed(&[market_signer!(
                ctx.base_mint.info.key(),
                ctx.quote_mint.info.key(),
                market_bump
            )])?;
        }
    }

    // If the user had any `quote_available`, transfer that amount from market account => user.
    if quote_available > 0 {
        if ctx.quote_token_program.is_spl_token {
            pinocchio_token::instructions::Transfer {
                from: ctx.market_quote_ata.info,      // WRITE
                to: ctx.user_quote_ata.info,          // WRITE
                authority: ctx.market_account.info(), // READ
                amount: quote_available,
            }
            .invoke_signed(&[market_signer!(
                ctx.base_mint.info.key(),
                ctx.quote_mint.info.key(),
                market_bump
            )])?;
        } else {
            // Safety: Scoped immutable borrow of mint account data to get mint decimals.
            let decimals = unsafe { ctx.quote_mint.get_mint_decimals() }?;
            pinocchio_token::instructions::TransferChecked {
                from: ctx.market_quote_ata.info,      // WRITE
                to: ctx.user_quote_ata.info,          // WRITE
                authority: ctx.market_account.info(), // READ
                mint: ctx.quote_mint.info,            // READ
                amount: quote_available,
                decimals,
            }
            .invoke_signed(&[market_signer!(
                ctx.base_mint.info.key(),
                ctx.quote_mint.info.key(),
                market_bump
            )])?;
        }
    }

    Ok(())
}
