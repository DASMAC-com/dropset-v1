use dropset_interface::{
    instructions::amount::AmountInstructionData,
    state::{market_seat::MarketSeat, node::Node, transmutable::load},
};
use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::pubkey_eq, ProgramResult,
};

use crate::{
    context::deposit_withdraw_context::DepositWithdrawContext,
    shared::{
        market_operations::{find_seat_with_hint, insert_market_seat},
        token_utils::market_transfers::deposit_to_market,
    },
};

pub fn process_deposit(accounts: &[AccountInfo], instruction_data: &[u8]) -> ProgramResult {
    let ctx = DepositWithdrawContext::load(accounts)?;

    // Safety: All bit patterns are valid.
    let args = unsafe { load::<AmountInstructionData>(instruction_data) }?;
    let amount = deposit_to_market(&ctx, args.amount())?;

    // Safety: Single immutable borrow of market account data.
    let market = unsafe { ctx.market_account.load_unchecked() }?;
    let needs_resize = market.header.num_free_sectors() == 0;

    let maybe_index = match args.sector_index_hint() {
        // User has provided a sector index hint; find the seat with it or fail and return early.
        Some(hint) => {
            // Return early if the hint provided was incorrect.
            let _ = find_seat_with_hint(market, hint, ctx.user.key())?;
            Some(hint)
        }
        None => market
            .iter_seats()
            .find(|(_, node)| pubkey_eq(&node.load_payload::<MarketSeat>().user, ctx.user.key()))
            .map(|(i, _)| i),
    };

    match maybe_index {
        // Update the user seat's available/deposited amounts for the corresponding mint type.
        Some(index) => {
            // Safety: Single mutable borrow of market account data, used to update the market seat.
            let market = unsafe { ctx.market_account.load_unchecked_mut() }?;
            // Safety: `i` is an in-bounds, non-NIL sector index, as it was just found.
            let seat = unsafe {
                Node::from_sector_index_mut_unchecked(market.sectors, index.get())
                    .load_payload_mut::<MarketSeat>()
            };
            if ctx.mint.is_base_mint {
                seat.set_base_available(
                    seat.base_available()
                        .checked_add(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                );
                seat.set_base_deposited(
                    seat.base_deposited()
                        .checked_add(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                );
            } else {
                seat.set_quote_available(
                    seat.quote_available()
                        .checked_add(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                );
                seat.set_quote_deposited(
                    seat.quote_deposited()
                        .checked_add(amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                );
            }
        }
        // Add space to the account if necessary and then insert a market seat into the list.
        None => {
            if needs_resize {
                // Safety: Single mutable borrow of market account data.
                unsafe { ctx.market_account.resize(ctx.user, 1) }?;
            }

            // Safety: Single mutable borrow of market account data.
            let mut market = unsafe { ctx.market_account.load_unchecked_mut() }?;

            let seat = if ctx.mint.is_base_mint {
                MarketSeat::new(*ctx.user.key(), amount, 0)
            } else {
                MarketSeat::new(*ctx.user.key(), 0, amount)
            };

            insert_market_seat(&mut market.seat_list(), seat)?;
        }
    }

    Ok(())
}
