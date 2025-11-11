//! Encapsulates the logic for safely transferring tokens to/from market accounts, enforcing
//! ownership and mint consistency guarantees.

use dropset_interface::{
    error::DropsetError,
    utils::is_owned_by_spl_token,
};
use pinocchio::{
    program_error::ProgramError,
    ProgramResult,
};

use crate::{
    context::deposit_withdraw_context::DepositWithdrawContext,
    market_signer,
};

/// Deposits `amount` of token `ctx.mint` from the user to the market account. This does not track
/// or update seat balances.
///
/// Returns an error if the amount deposited is zero.
///
/// # Safety
///
/// Caller guarantees:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Accounts
///   0. `[WRITE]` User token account (source)
///   1. `[WRITE]` Market token account (destination)
///   2. `[READ]` User account (authority)
///   3. `[READ]` Mint account
pub unsafe fn deposit_non_zero_to_market(
    ctx: &DepositWithdrawContext,
    amount: u64,
) -> Result<u64, ProgramError> {
    let amount_deposited = if is_owned_by_spl_token(ctx.mint.info) {
        pinocchio_token::instructions::Transfer {
            from: ctx.user_ata.info, // WRITE
            to: ctx.market_ata.info, // WRITE
            authority: ctx.user,     // READ
            amount,
        }
        .invoke()?;

        // `spl_token` always transfers the exact amount passed in.
        amount
    } else {
        // Safety: Scoped immutable borrow to read the mint account's mint decimals.
        let decimals = unsafe { ctx.mint.get_mint_decimals() }?;

        // Safety: Scoped immutable borrow of the market token account data to get its balance.
        let balance_before = unsafe { ctx.market_ata.get_balance() }?;

        pinocchio_token_2022::instructions::TransferChecked {
            from: ctx.user_ata.info, // WRITE
            to: ctx.market_ata.info, // WRITE
            mint: ctx.mint.info,     // READ
            authority: ctx.user,     // READ
            decimals,
            amount,
            token_program: &pinocchio_token_2022::ID,
        }
        .invoke()?;

        // Safety: Scoped immutable borrow of the market token account data to get its balance.
        let balance_after = unsafe { ctx.market_ata.get_balance() }?;

        // `spl_token_2022` amount deposited must be checked due to transfer hooks, fees, and other
        // extensions that may intercept a simple transfer and alter the amount transferred.
        balance_after
            .checked_sub(balance_before)
            .ok_or(ProgramError::InvalidArgument)?
    };

    if amount_deposited > 0 {
        Ok(amount_deposited)
    } else {
        Err(DropsetError::AmountCannotBeZero.into())
    }
}

/// Withdraws `amount` of token `ctx.mint` from the market account to the user. This does not track
/// or update seat balances.
///
/// Returns an error if the amount withdrawn is zero.
///
/// # Safety
///
/// Caller guarantees:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Accounts
///   0. `[WRITE]` User token account (destination)
///   1. `[WRITE]` Market token account (source)
///   2. `[READ]`  Market account (authority)
///   3. `[READ]`  Mint account
pub unsafe fn withdraw_non_zero_from_market(
    ctx: &DepositWithdrawContext,
    amount: u64,
) -> ProgramResult {
    if amount == 0 {
        return Err(DropsetError::AmountCannotBeZero.into());
    }

    let (base_mint, quote_mint, market_bump) = {
        // Safety: Scoped immutable borrow of the market account.
        let market = unsafe { ctx.market_account.load_unchecked() };
        (
            market.header.base_mint,
            market.header.quote_mint,
            market.header.market_bump,
        )
    };

    if is_owned_by_spl_token(ctx.mint.info) {
        pinocchio_token::instructions::Transfer {
            from: ctx.market_ata.info,            // WRITE
            to: ctx.user_ata.info,                // WRITE
            authority: ctx.market_account.info(), // READ
            amount,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    } else {
        // Safety: Scoped immutable borrow of mint account data to get the mint decimals.
        let decimals = unsafe { ctx.mint.get_mint_decimals() }?;

        pinocchio_token_2022::instructions::TransferChecked {
            from: ctx.market_ata.info,            // WRITE
            to: ctx.user_ata.info,                // WRITE
            mint: ctx.mint.info,                  // READ
            authority: ctx.market_account.info(), // READ
            amount,
            decimals,
            token_program: &pinocchio_token_2022::ID,
        }
        .invoke_signed(&[market_signer!(base_mint, quote_mint, market_bump)])
    }
}
