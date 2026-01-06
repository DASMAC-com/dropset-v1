//! Definitions and re-exports of all program instructions, organized for both client-side and
//! on-chain usage.
//!
//! This module re-exports proc-macro generated code in multiple forms, depending on which features
//! are enabled.
//!
//! The `pinocchio` feature: [`crate::instructions::generated_pinocchio`]
//! The `client` feature: [`crate::instructions::generated_client`]
//!
//! The `solana-sdk` feature is disabled but enables `crate::instructions::generated_solana_sdk` for
//! use with non-pinocchio based programs.
//!
//! This module also re-exports the [`DropsetInstruction_try_from_tag`] macro.

use instruction_macros::ProgramInstruction;

use crate::error::DropsetError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, ProgramInstruction)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
#[cfg_attr(feature = "client", derive(strum_macros::Display))]
#[program_id(crate::program::ID)]
#[rustfmt::skip]
pub enum DropsetInstruction {
    #[account(0,           name = "event_authority",      desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",                 desc = "The user closing their seat.")]
    #[account(2, writable, name = "market_account",       desc = "The market account PDA.")]
    #[account(3, writable, name = "base_user_ata",        desc = "The user's associated base token account.")]
    #[account(4, writable, name = "quote_user_ata",       desc = "The user's associated quote token account.")]
    #[account(5, writable, name = "base_market_ata",      desc = "The market's associated base token account.")]
    #[account(6, writable, name = "quote_market_ata",     desc = "The market's associated quote token account.")]
    #[account(7,           name = "base_mint",            desc = "The base token mint account.")]
    #[account(8,           name = "quote_mint",           desc = "The quote token mint account.")]
    #[account(9,           name = "base_token_program",   desc = "The base mint's token program.")]
    #[account(10,          name = "quote_token_program",  desc = "The quote mint's token program.")]
    #[account(11,          name = "dropset_program",      desc = "The dropset program itself, used for the self-CPI.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    CloseSeat,

    #[account(0,           name = "event_authority", desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",            desc = "The user depositing or registering their seat.")]
    #[account(2, writable, name = "market_account",  desc = "The market account PDA.")]
    #[account(3, writable, name = "user_ata",        desc = "The user's associated token account.")]
    #[account(4, writable, name = "market_ata",      desc = "The market's associated token account.")]
    #[account(5,           name = "mint",            desc = "The token mint account.")]
    #[account(6,           name = "token_program",   desc = "The mint's token program.")]
    #[account(7,           name = "dropset_program", desc = "The dropset program itself, used for the self-CPI.")]
    #[args(amount: u64, "The amount to deposit.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in (pass `NIL` when registering a new seat).")]
    Deposit,

    #[account(0,           name = "event_authority",     desc = "The event authority PDA signer.")]
    #[account(1, signer, writable, name = "user",        desc = "The user registering the market.")]
    #[account(2, writable, name = "market_account",      desc = "The market account PDA.")]
    #[account(3, writable, name = "base_market_ata",     desc = "The market's associated token account for the base mint.")]
    #[account(4, writable, name = "quote_market_ata",    desc = "The market's associated token account for the quote mint.")]
    #[account(5,           name = "base_mint",           desc = "The base token mint account.")]
    #[account(6,           name = "quote_mint",          desc = "The quote token mint account.")]
    #[account(7,           name = "base_token_program",  desc = "The base mint's token program.")]
    #[account(8,           name = "quote_token_program", desc = "The quote mint's token program.")]
    #[account(9,           name = "ata_program",         desc = "The associated token account program.")]
    #[account(10,          name = "system_program",      desc = "The system program.")]
    #[account(11,          name = "dropset_program",     desc = "The dropset program itself, used for the self-CPI.")]
    #[args(num_sectors: u16, "The number of sectors to preallocate for the market.")]
    RegisterMarket,

    #[account(0,           name = "event_authority", desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",            desc = "The user withdrawing.")]
    #[account(2, writable, name = "market_account",  desc = "The market account PDA.")]
    #[account(3, writable, name = "user_ata",        desc = "The user's associated token account.")]
    #[account(4, writable, name = "market_ata",      desc = "The market's associated token account.")]
    #[account(5,           name = "mint",            desc = "The token mint account.")]
    #[account(6,           name = "token_program",   desc = "The mint's token program.")]
    #[account(7,           name = "dropset_program", desc = "The dropset program itself, used for the self-CPI.")]
    #[args(amount: u64, "The amount to withdraw.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    Withdraw,

    #[account(0,           name = "event_authority", desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",            desc = "The user posting an order.")]
    #[account(2, writable, name = "market_account",  desc = "The market account PDA.")]
    #[account(3,           name = "dropset_program", desc = "The dropset program itself, used for the self-CPI.")]
    #[args(price_mantissa: u32, "The price mantissa.")]
    #[args(base_scalar: u64, "The scalar for the base token.")]
    #[args(base_exponent_biased: u8, "The biased base exponent.")]
    #[args(quote_exponent_biased: u8, "The biased quote exponent.")]
    #[args(is_bid: bool, "Whether or not the order is a bid. If false, the order is an ask.")]
    #[args(user_sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    PostOrder,

    #[account(0,           name = "event_authority", desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",            desc = "The user canceling an order.")]
    #[account(2, writable, name = "market_account",  desc = "The market account PDA.")]
    #[account(3,           name = "dropset_program", desc = "The dropset program itself, used for the self-CPI.")]
    #[args(encoded_price: u32, "The encoded price for the order to cancel.")]
    #[args(is_bid: bool, "Whether or not the order is a bid. If false, the order is an ask.")]
    #[args(user_sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    CancelOrder,

    #[account(0,           name = "event_authority",     desc = "The event authority PDA signer.")]
    #[account(1, signer,   name = "user",                desc = "The user creating the market order, aka the taker.")]
    #[account(2, writable, name = "market_account",      desc = "The market account PDA.")]
    #[account(3, writable, name = "base_user_ata",       desc = "The user's associated base token account.")]
    #[account(4, writable, name = "quote_user_ata",      desc = "The user's associated quote token account.")]
    #[account(5, writable, name = "base_market_ata",     desc = "The market's associated base token account.")]
    #[account(6, writable, name = "quote_market_ata",    desc = "The market's associated quote token account.")]
    #[account(7,           name = "base_mint",           desc = "The base token mint account.")]
    #[account(8,           name = "quote_mint",          desc = "The quote token mint account.")]
    #[account(9,           name = "base_token_program",  desc = "The base mint's token program.")]
    #[account(10,          name = "quote_token_program", desc = "The quote mint's token program.")]
    #[account(11,          name = "dropset_program",     desc = "The dropset program itself, used for the self-CPI.")]
    #[args(order_size: u64, "The order size; aka the number of atoms to fill.")]
    #[args(is_buy: bool, "Whether or not the order is a market buy. If not, it's a market sell.")]
    #[args(is_base: bool, "Whether or not the order size is denominated in base. If not, it's in quote.")]
    MarketOrder,

    // FlushEvents is an internal instruction and can only be called by the program. It does have
    // instruction data, but it is not used by the program.
    #[account(0, signer,   name = "event_authority", desc = "The event authority PDA signer.")]
    FlushEvents,

    Batch,
}

impl TryFrom<u8> for DropsetInstruction {
    type Error = DropsetError;

    #[inline(always)]
    fn try_from(tag: u8) -> Result<Self, Self::Error> {
        DropsetInstruction_try_from_tag!(tag, DropsetError::InvalidInstructionTag)
    }
}

#[cfg(test)]
mod test {
    use strum::IntoEnumIterator;

    extern crate std;
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn test_ixn_tag_try_from_u8_happy_path() {
        for variant in DropsetInstruction::iter() {
            let variant_u8 = variant as u8;
            assert_eq!(
                DropsetInstruction::from_repr(variant_u8).unwrap(),
                DropsetInstruction::try_from(variant_u8).unwrap(),
            );
            assert_eq!(DropsetInstruction::try_from(variant_u8).unwrap(), variant);
        }
    }

    #[test]
    fn test_ixn_tag_try_from_u8_exhaustive() {
        let valids = DropsetInstruction::iter()
            .map(|v| v as u8)
            .collect::<HashSet<_>>();

        for v in 0..=u8::MAX {
            if valids.contains(&v) {
                assert_eq!(
                    DropsetInstruction::from_repr(v).unwrap(),
                    DropsetInstruction::try_from(v).unwrap(),
                );
                assert_eq!(DropsetInstruction::try_from(v).unwrap() as u8, v);
            } else {
                assert_eq!(
                    DropsetInstruction::from_repr(v).is_none(),
                    DropsetInstruction::try_from(v).is_err(),
                );
            }
        }
    }
}
