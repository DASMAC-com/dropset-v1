use dropset_interface::state::market::MarketRef;
use pinocchio::{program_error::ProgramError, ProgramResult};

use crate::{context::deposit_withdraw_context::DepositWithdrawContext, market_signer};

/// Deposits `amount` of token `ctx.mint` from the user to the market account. This does not track
/// or update seat balances.
///
/// # Safety
///
/// Caller guarantees the market token account is not currently borrowed.
pub fn deposit_to_market(ctx: &DepositWithdrawContext, amount: u64) -> Result<u64, ProgramError> {
    if ctx.token_program.is_spl_token {
        pinocchio_token::instructions::Transfer {
            from: ctx.user_ata.info,
            to: ctx.market_ata.info,
            authority: ctx.user,
            amount,
        }
        .invoke()?;

        // `spl_token` always transfers the exact amount passed in.
        Ok(amount)
    } else {
        let decimals = ctx.mint.get_mint_decimals()?;

        // Safety: Single, scoped borrow of the market token account data to get its balance.
        let balance_before = unsafe { ctx.market_ata.get_balance() }?;

        pinocchio_token_2022::instructions::TransferChecked {
            from: ctx.user_ata.info,
            to: ctx.market_ata.info,
            mint: ctx.mint.info,
            authority: ctx.user,
            decimals,
            amount,
            token_program: ctx.token_program.info.key(),
        }
        .invoke()?;

        // Safety: Single, scoped borrow of the market token account data to get its balance.
        let balance_after = unsafe { ctx.market_ata.get_balance() }?;

        // `spl_token_2022` amount deposited must be checked due to transfer hooks, fees, and other
        // extensions that may intercept a simple transfer and alter the amount transferred.
        let deposited = balance_after
            .checked_sub(balance_before)
            .ok_or(ProgramError::InvalidArgument)?;
        Ok(deposited)
    }
}

/// Withdraws `amount` of token `ctx.mint` from the market account to the user. This does not track
/// or update seat balances.
///
/// # Safety
///
/// Caller guarantees the market account is not currently borrowed.
pub fn withdraw_from_market(ctx: &DepositWithdrawContext, amount: u64) -> ProgramResult {
    let (base_mint, quote_mint, market_bump) = {
        // Safety: Scoped immutable borrow to copy the signer seeds necessary.
        let market = unsafe { ctx.market_account.load_unchecked() };
        (
            market.header.base_mint,
            market.header.quote_mint,
            market.header.market_bump,
        )
    };

    if ctx.token_program.is_spl_token {
        pinocchio_token::instructions::Transfer {
            from: ctx.market_ata.info,
            to: ctx.user_ata.info,
            authority: ctx.market_account.info(),
            amount,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    } else {
        let decimals = ctx.mint.get_mint_decimals()?;
        pinocchio_token::instructions::TransferChecked {
            from: ctx.market_ata.info,
            to: ctx.user_ata.info,
            authority: ctx.market_account.info(),
            amount,
            mint: ctx.mint.info,
            decimals,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    }
}
