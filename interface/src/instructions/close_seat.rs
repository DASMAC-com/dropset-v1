// use pinocchio::{
//     account_info::AccountInfo,
//     instruction::{AccountMeta, Instruction, Signer},
//     ProgramResult,
// };

// use crate::{
//     instructions::InstructionTag,
//     pack::{write_bytes, UNINIT_BYTE},
//     state::sector::SectorIndex,
// };
// /// Closes a market seat for a user by withdrawing all base and quote from their seat.
// ///
// /// # Safety
// ///
// /// Caller guarantees:
// /// - WRITE accounts are not currently borrowed in *any* capacity.
// /// - READ accounts are not currently mutably borrowed.
// ///
// /// ### Accounts
// ///   0. `[WRITE]` Market account
// ///   1. `[WRITE]` Market base mint token account
// ///   2. `[WRITE]` Market quote mint token account
// ///   3. `[WRITE]` User base mint token account
// ///   4. `[WRITE]` User quote mint token account
// ///   5. `[READ]` Base mint
// ///   6. `[READ]` Quote mint
// pub struct Close<'a> {

// }
