use dropset_interface::{
    pack::unpack_u32,
    state::{node::Node, sector::SectorIndex},
    utils::is_owned_by_spl_token,
};
use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::{
    context::close_seat_context::CloseSeatContext, market_signer,
    shared::market_operations::find_seat_with_hint,
};

/// Closes a market seat for a user by withdrawing all base and quote from their seat.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in [`dropset_interface::instructions::close_seat::CloseSeat`]
pub fn process_close_seat(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let hint = SectorIndex(unpack_u32(instruction_data)?);
    let mut ctx = unsafe { CloseSeatContext::load(accounts) }?;

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
        if is_owned_by_spl_token(ctx.base_mint.info) {
            pinocchio_token::instructions::Transfer {
                from: ctx.base_market_ata.info,       // WRITE
                to: ctx.base_user_ata.info,           // WRITE
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
            pinocchio_token_2022::instructions::TransferChecked {
                from: ctx.base_market_ata.info,       // WRITE
                to: ctx.base_user_ata.info,           // WRITE
                authority: ctx.market_account.info(), // READ
                mint: ctx.base_mint.info,             // READ
                amount: base_available,
                decimals,
                token_program: &pinocchio_token_2022::ID,
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
        if is_owned_by_spl_token(ctx.quote_mint.info) {
            pinocchio_token::instructions::Transfer {
                from: ctx.quote_market_ata.info,      // WRITE
                to: ctx.quote_user_ata.info,          // WRITE
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
            pinocchio_token_2022::instructions::TransferChecked {
                from: ctx.quote_market_ata.info,      // WRITE
                to: ctx.quote_user_ata.info,          // WRITE
                authority: ctx.market_account.info(), // READ
                mint: ctx.quote_mint.info,            // READ
                amount: quote_available,
                decimals,
                token_program: &pinocchio_token_2022::ID,
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
