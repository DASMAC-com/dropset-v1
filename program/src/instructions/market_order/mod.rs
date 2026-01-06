//! See [`process_market_order`].

mod fill_market_order;
mod mul_div_checked;

// #[cfg(feature = "debug")]
use dropset_interface::{
    error::DropsetError,
    events::MarketOrderEventInstructionData,
    instructions::MarketOrderInstructionData,
};
use mul_div_checked::mul_div_checked;
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
};

use crate::{
    context::{
        market_order_context::MarketOrderContext,
        EventBufferContext,
    },
    events::EventBuffer,
    instructions::market_order::fill_market_order::{
        fill_market_order,
        AmountsFilled,
    },
    shared::token_utils::market_transfers::{
        deposit_non_zero_to_market,
        withdraw_non_zero_from_market,
    },
};

/// Instruction handler logic for processing a market order.
///
/// # Safety
///
/// Caller guarantees the safety contract detailed in
/// [`dropset_interface::instructions::generated_pinocchio::MarketOrder`].
#[inline(never)]
pub unsafe fn process_market_order<'a>(
    accounts: &'a [AccountInfo],
    instruction_data: &[u8],
    _event_buffer: &mut EventBuffer,
) -> Result<EventBufferContext<'a>, ProgramError> {
    let MarketOrderInstructionData {
        order_size,
        is_buy,
        is_base,
    } = MarketOrderInstructionData::unpack_pinocchio(instruction_data)?;
    let mut ctx = MarketOrderContext::load(accounts)?;

    let AmountsFilled {
        base: base_filled,
        quote: quote_filled,
    } = match (is_buy, is_base) {
        (false, false) => fill_market_order::<false, false>(&mut ctx, order_size),
        (true, false) => fill_market_order::<true, false>(&mut ctx, order_size),
        (false, true) => fill_market_order::<false, true>(&mut ctx, order_size),
        (true, true) => fill_market_order::<true, true>(&mut ctx, order_size),
    }?;

    // Try to transfer the taker side's tokens to the market account.
    // Safety: No account data is currently borrowed.
    let (taker_amount_filled, taker_amount_deposited) = unsafe {
        // A buy means taker transfers quote to the market.
        if is_buy {
            let quote_transferred = deposit_non_zero_to_market(
                &ctx.quote_user_ata,
                &ctx.quote_market_ata,
                ctx.user,
                &ctx.quote_mint,
                quote_filled,
            )?;

            // And receives base.
            withdraw_non_zero_from_market(
                &ctx.base_user_ata,
                &ctx.base_market_ata,
                &ctx.market_account,
                &ctx.base_mint,
                base_filled,
            )?;

            (quote_filled, quote_transferred)
        // A sell means taker transfers base to the market.
        } else {
            let base_transferred = deposit_non_zero_to_market(
                &ctx.base_user_ata,
                &ctx.base_market_ata,
                ctx.user,
                &ctx.base_mint,
                base_filled,
            )?;

            // And receives quote.
            withdraw_non_zero_from_market(
                &ctx.quote_user_ata,
                &ctx.quote_market_ata,
                &ctx.market_account,
                &ctx.quote_mint,
                quote_filled,
            )?;

            (base_filled, base_transferred)
        }
    };

    // Ensure that the order size matches the exact amount transferred.
    if taker_amount_filled != taker_amount_deposited {
        return Err(DropsetError::AmountFilledVsTransferredMismatch.into());
    }

    // #[cfg(feature = "debug")]
    _event_buffer.add_to_buffer(
        MarketOrderEventInstructionData::new(
            order_size,
            is_buy,
            is_base,
            base_filled,
            quote_filled,
        ),
        ctx.event_authority,
        ctx.market_account.clone(),
    )?;

    Ok(EventBufferContext {
        event_authority: ctx.event_authority,
        market_account: ctx.market_account,
    })
}
