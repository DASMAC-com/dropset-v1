use pinocchio::ProgramResult;

use crate::context::register_market_context::RegisterMarketContext;

/// Creates the base and quote associated token accounts for a market.
pub fn create_atas(ctx: &RegisterMarketContext) -> ProgramResult {
    for (mint, ata, token_program) in [
        (
            ctx.base_mint,
            ctx.base_market_ata,
            ctx.base_token_program.info,
        ),
        (
            ctx.quote_mint,
            ctx.quote_market_ata,
            ctx.quote_token_program.info,
        ),
    ] {
        // Create the associated token accounts with the non-idempotent instruction to ensure that
        // passing duplicate mint accounts fails.
        pinocchio_associated_token_account::instructions::Create {
            funding_account: ctx.user,
            account: ata,
            wallet: ctx.market_account.info,
            mint,
            system_program: ctx.system_program,
            token_program,
        }
        .invoke()?;
    }

    Ok(())
}
