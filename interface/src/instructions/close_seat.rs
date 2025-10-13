use instruction_macros::ProgramInstructions;
use pinocchio::{
    instruction::{Instruction, Signer},
    ProgramResult,
};

use crate::{
    instructions::InstructionTag,
    pack::{write_bytes, UNINIT_BYTE},
    state::sector::SectorIndex,
};

pub use pinocchio::account_info::AccountInfo;

use pinocchio::instruction::AccountMeta;

#[derive(ProgramInstructions)]
#[rustfmt::skip]
pub enum DropsetInstruction {
    #[account(0, writable, signer, name = "my_account", desc = "The user closing their seat")]
    #[account(1, name = "acc2")]
    #[account(2, name = "acc1")]
    #[account(3, name = "acc0")]
    CloseSeat,

    #[account(0, signer, name = "acc0", desc = "The user depositing tokens")]
    #[account(1, signer, name = "acc1")]
    #[account(2, signer, name = "acc3")]
    #[account(3, signer, name = "acc2")]
    #[account(4, signer, name = "acc4")]
    #[args(my_first_arg: u64)]
    Deposit,
}

/// Closes a market seat for a user by withdrawing all base and quote from their seat.
///
/// # Caller guarantees
///
/// When invoking this instruction, caller must ensure that:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Accounts
///   0. `[READ, SIGNER]` User
///   1. `[WRITE]` Market account
///   2. `[WRITE]` User base mint token account
///   3. `[WRITE]` User quote mint token account
///   4. `[WRITE]` Market base mint token account
///   5. `[WRITE]` Market quote mint token account
///   6. `[READ]` Base mint
///   7. `[READ]` Quote mint
pub struct CloseSeat<'a> {
    /// The user closing their seat.
    pub user: &'a AccountInfo,
    /// The market account PDA.
    pub market_account: &'a AccountInfo,
    /// The user's associated base mint token account.
    pub base_user_ata: &'a AccountInfo,
    /// The user's associated quote mint token account.
    pub quote_user_ata: &'a AccountInfo,
    /// The market's associated base mint token account.
    pub base_market_ata: &'a AccountInfo,
    /// The market's associated quote mint token account.
    pub quote_market_ata: &'a AccountInfo,
    /// The base token mint account.
    pub base_mint: &'a AccountInfo,
    /// The quote token mint account.
    pub quote_mint: &'a AccountInfo,
    /// A hint indicating which sector index the user's seat is at in the sectors array.
    pub sector_index_hint: SectorIndex,
}

impl CloseSeat<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[inline(always)]
    pub fn invoke_signed(&self, signers_seeds: &[Signer]) -> ProgramResult {
        pinocchio::cpi::invoke_signed(
            &Instruction {
                program_id: &crate::program::ID,
                accounts: &[
                    AccountMeta::readonly_signer(self.user.key()),
                    AccountMeta::writable(self.market_account.key()),
                    AccountMeta::writable(self.base_user_ata.key()),
                    AccountMeta::writable(self.quote_user_ata.key()),
                    AccountMeta::writable(self.base_market_ata.key()),
                    AccountMeta::writable(self.quote_market_ata.key()),
                    AccountMeta::readonly(self.base_mint.key()),
                    AccountMeta::readonly(self.quote_mint.key()),
                ],
                data: &self.pack(),
            },
            &[
                self.user,
                self.market_account,
                self.base_user_ata,
                self.quote_user_ata,
                self.base_market_ata,
                self.quote_market_ata,
                self.base_mint,
                self.quote_mint,
            ],
            signers_seeds,
        )
    }

    // #[cfg(feature = "client")]
    // pub fn create_account_metas(&self) -> [AccountMeta; 8] {
    //     [
    //         AccountMeta::new_readonly((*self.user).into(), true),
    //         AccountMeta::new((*self.market_account).into(), false),
    //         AccountMeta::new((*self.base_user_ata).into(), false),
    //         AccountMeta::new((*self.quote_user_ata).into(), false),
    //         AccountMeta::new((*self.base_market_ata).into(), false),
    //         AccountMeta::new((*self.quote_market_ata).into(), false),
    //         AccountMeta::new_readonly((*self.base_mint).into(), false),
    //         AccountMeta::new_readonly((*self.quote_mint).into(), false),
    //     ]
    // }

    #[inline(always)]
    pub fn pack(&self) -> [u8; 5] {
        // Instruction data layout:
        //   - [0]: the instruction tag, 1 byte
        //   - [1..5]: the u32 `sector_index_hint` as little-endian bytes, 4 bytes
        let mut data = [UNINIT_BYTE; 5];

        data[0].write(InstructionTag::CloseSeat as u8);
        write_bytes(&mut data[1..5], &self.sector_index_hint.0.to_le_bytes());

        // Safety: All 5 bytes were written to.
        unsafe { *(data.as_ptr() as *const _) }
    }
}
