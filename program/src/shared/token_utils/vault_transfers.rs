use pinocchio::{program_error::ProgramError, ProgramResult};

use crate::context::deposit_withdraw_context::DepositWithdrawContext;

pub fn deposit_to_vault(ctx: &DepositWithdrawContext, amount: u64) -> Result<u64, ProgramError> {
    if ctx.token_program.is_spl_token {
        pinocchio_token::instructions::Transfer {
            from: ctx.user_ata.info,
            to: ctx.vault_ata.info,
            authority: ctx.user,
            amount,
        }
        .invoke()?;
        // `spl_token` always transfers the exact amount passed in.
        Ok(amount)
    } else {
        let decimals = ctx.mint.get_mint_decimals()?;

        let balance_before = ctx.vault_ata.get_balance()?;

        pinocchio_token_2022::instructions::TransferChecked {
            from: ctx.user_ata.info,
            to: ctx.vault_ata.info,
            mint: ctx.mint.info,
            authority: ctx.user,
            decimals,
            amount,
            token_program: ctx.token_program.info.key(),
        }
        .invoke()?;
        let balance_after = ctx.vault_ata.get_balance()?;
        // `spl_token_2022` amount deposited must be checked due to transfer hooks, fees, and other
        // extensions that may intercept a simple transfer and alter the amount transferred.
        let deposited = balance_after
            .checked_sub(balance_before)
            .ok_or(ProgramError::InvalidArgument)?;
        Ok(deposited)
    }
}

pub fn withdraw_from_vault(_ctx: &DepositWithdrawContext, _amount: u64) -> ProgramResult {
    todo!()
}
