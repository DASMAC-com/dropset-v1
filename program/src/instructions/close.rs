use dropset_interface::{instructions::close::CloseInstructionData, state::transmutable::load};
use pinocchio::{account_info::AccountInfo, ProgramResult};

use crate::{
    context::close_context::CloseContext, market_signer,
    shared::market_operations::find_mut_seat_with_hint,
};

pub fn process_close(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let ctx = CloseContext::load(accounts)?;

    // Safety: All bit patterns are valid.
    let args = unsafe { load::<CloseInstructionData>(instruction_data) }?;
    let hint = args.try_sector_index_hint()?;

    // Safety: Single mutable borrow of market account data.
    let market = unsafe { ctx.market_account.load_unchecked_mut() }?;

    // Get the base and quote amounts available for the user,
    // NOTE: The base/quote available and deposited do not need to be zeroed here because they're
    // zeroed out in the `push_free_node` call in the `remove_at` method below.
    let seat = find_mut_seat_with_hint(market, hint, ctx.user.key())?;
    let (base_available, quote_available) = (seat.base_available(), seat.quote_available());

    // Safety: Single mutable borrow of market account data.
    let mut market = unsafe { ctx.market_account.load_unchecked_mut() }?;

    // Remove the seat/node, push it to the free stack, and zero it out.
    // Safety: The index hint is definitively valid because it was used to find the seat.
    unsafe { market.seat_list().remove_at(hint) }?;

    let market_bump = market.header.market_bump;

    // Now withdraw from both base and quote.
    for (from, to, amount, is_spl_token, mint) in [
        (
            ctx.market_base_ata.info,
            ctx.user_base_ata.info,
            base_available,
            ctx.base_token_program.is_spl_token,
            ctx.base_mint.clone(),
        ),
        (
            ctx.market_quote_ata.info,
            ctx.user_quote_ata.info,
            quote_available,
            ctx.quote_token_program.is_spl_token,
            ctx.quote_mint.clone(),
        ),
    ] {
        if amount > 0 {
            if is_spl_token {
                pinocchio_token::instructions::Transfer {
                    from,
                    to,
                    authority: ctx.market_account.info,
                    amount,
                }
                .invoke_signed(&[market_signer!(
                    ctx.base_mint.info.key(),
                    ctx.quote_mint.info.key(),
                    market_bump
                )])?;
            } else {
                let decimals = mint.get_mint_decimals()?;
                pinocchio_token::instructions::TransferChecked {
                    from,
                    to,
                    authority: ctx.market_account.info,
                    amount,
                    mint: mint.info,
                    decimals,
                }
                .invoke_signed(&[market_signer!(
                    ctx.base_mint.info.key(),
                    ctx.quote_mint.info.key(),
                    market_bump
                )])?;
            }
        }
    }

    Ok(())
}
