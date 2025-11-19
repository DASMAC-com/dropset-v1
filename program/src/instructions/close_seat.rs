//! See [`process_close_seat`].

use dropset_interface::{
    events::CloseSeatEventInstructionData,
    instructions::generated_pinocchio::*,
    state::node::Node,
    utils::is_owned_by_spl_token,
};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::{
    context::{
        close_seat_context::CloseSeatContext,
        EventBufferContext,
    },
    events::EventBuffer,
    market_signer,
    shared::market_operations::find_seat_with_hint,
};

/// Instruction handler logic for closing an existing market seat and reclaiming associated funds.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in [`CloseSeat`].
#[inline(never)]
pub fn process_close_seat<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
    event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let sector_index_hint = CloseSeatInstructionData::unpack(instruction_data)?.sector_index_hint;
    let mut ctx = unsafe { CloseSeatContext::load(accounts) }?;

    // Get the market bump and the base and quote amounts available for the user.
    let (market_bump, base_available, quote_available) = unsafe {
        // Safety: Scoped immutable borrow of market account data.
        let market = ctx.market_account.load_unchecked();
        let market_bump = market.header.market_bump;

        Node::check_in_bounds(market.sectors, sector_index_hint)?;
        // Safety: The index hint was just verified as in-bounds.
        let seat = find_seat_with_hint(market, sector_index_hint, ctx.user.key())?;

        // NOTE: The base/quote available and deposited do not need to be zeroed here because
        // they're zeroed out in the `push_free_node` call in the `remove_at` method below.
        (market_bump, seat.base_available(), seat.quote_available())
    };

    // Remove the seat, push it to the free stack, and zero it out.
    unsafe {
        ctx.market_account
            // Safety: Scoped mutable borrow of market account data to remove the seat.
            .load_unchecked_mut()
            .seat_list()
            // Safety: The index hint was verified as in-bounds.
            .remove_at(sector_index_hint)
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

    event_buffer.add_to_buffer(
        CloseSeatEventInstructionData::new(sector_index_hint),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
