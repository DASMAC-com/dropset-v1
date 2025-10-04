use dropset_interface::state::market::MarketRef;
use pinocchio::{program_error::ProgramError, ProgramResult};

use crate::{context::deposit_withdraw_context::DepositWithdrawContext, market_signer};

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
        let balance_before = ctx.market_ata.get_balance()?;

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

        let balance_after = ctx.market_ata.get_balance()?;
        // `spl_token_2022` amount deposited must be checked due to transfer hooks, fees, and other
        // extensions that may intercept a simple transfer and alter the amount transferred.
        let deposited = balance_after
            .checked_sub(balance_before)
            .ok_or(ProgramError::InvalidArgument)?;
        Ok(deposited)
    }
}

pub fn withdraw_from_market(ctx: &DepositWithdrawContext, amount: u64) -> ProgramResult {
    let (base_mint, quote_mint, market_bump) = {
        // Safety: Single immutable borrow to the market account data.
        let data = unsafe { ctx.market_account.info.borrow_data_unchecked() };
        let market = MarketRef::from_bytes_unchecked(data)?;
        let header = market.header;
        (header.base_mint, header.quote_mint, header.market_bump)
    };

    if ctx.token_program.is_spl_token {
        pinocchio_token::instructions::Transfer {
            from: ctx.market_ata.info,
            to: ctx.user_ata.info,
            authority: ctx.market_account.info,
            amount,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    } else {
        let decimals = ctx.mint.get_mint_decimals()?;
        pinocchio_token::instructions::TransferChecked {
            from: ctx.market_ata.info,
            to: ctx.user_ata.info,
            authority: ctx.market_account.info,
            amount,
            mint: ctx.mint.info,
            decimals,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    }
}
