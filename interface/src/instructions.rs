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
    #[account(0, signer,   name = "user",                desc = "The user closing their seat.")]
    #[account(1, writable, name = "market_account",      desc = "The market account PDA.")]
    #[account(2, writable, name = "base_user_ata",       desc = "The user's associated base mint token account.")]
    #[account(3, writable, name = "quote_user_ata",      desc = "The user's associated quote mint token account.")]
    #[account(4, writable, name = "base_market_ata",     desc = "The market's associated base mint token account.")]
    #[account(5, writable, name = "quote_market_ata",    desc = "The market's associated quote mint token account.")]
    #[account(6,           name = "base_mint",           desc = "The base token mint account.")]
    #[account(7,           name = "quote_mint",          desc = "The quote token mint account.")]
    #[account(8,           name = "base_token_program",  desc = "The base mint's token program.")]
    #[account(9,           name = "quote_token_program", desc = "The quote mint's token program.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    CloseSeat,

    #[account(0, signer,   name = "user",           desc = "The user depositing or registering their seat.")]
    #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
    #[account(2, writable, name = "user_ata",       desc = "The user's associated token account.")]
    #[account(3, writable, name = "market_ata",     desc = "The market's associated token account.")]
    #[account(4,           name = "mint",           desc = "The token mint account.")]
    #[account(5,           name = "token_program",  desc = "The mint's token program.")]
    #[args(amount: u64, "The amount to deposit.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in (pass `NIL` when registering a new seat).")]
    Deposit,

    #[account(0, signer, writable, name = "user",        desc = "The user registering the market.")]
    #[account(1, writable, name = "market_account",      desc = "The market account PDA.")]
    #[account(2, writable, name = "base_market_ata",     desc = "The market's associated token account for the base mint.")]
    #[account(3, writable, name = "quote_market_ata",    desc = "The market's associated token account for the quote mint.")]
    #[account(4,           name = "base_mint",           desc = "The base mint account.")]
    #[account(5,           name = "quote_mint",          desc = "The quote mint account.")]
    #[account(6,           name = "base_token_program",  desc = "The base mint's token program.")]
    #[account(7,           name = "quote_token_program", desc = "The quote mint's token program.")]
    #[account(8,           name = "ata_program",         desc = "The associated token account program.")]
    #[account(9,           name = "system_program",      desc = "The system program.")]
    #[args(num_sectors: u16, "The number of sectors to preallocate for the market.")]
    RegisterMarket,

    #[account(0, signer,   name = "user",           desc = "The user withdrawing.")]
    #[account(1, writable, name = "market_account", desc = "The market account PDA.")]
    #[account(2, writable, name = "user_ata",       desc = "The user's associated token account.")]
    #[account(3, writable, name = "market_ata",     desc = "The market's associated token account.")]
    #[account(4,           name = "mint",           desc = "The token mint account.")]
    #[account(5,           name = "token_program",  desc = "The mint's token program.")]
    #[args(amount: u64, "The amount to withdraw.")]
    #[args(sector_index_hint: u32, "A hint indicating which sector the user's seat resides in.")]
    Withdraw,

    #[account(0, signer, name = "event_authority", desc = "Flush events.")]
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
