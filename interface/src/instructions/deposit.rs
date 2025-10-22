use pinocchio::{
    account_info::AccountInfo,
    instruction::{
        AccountMeta,
        Instruction,
        Signer,
    },
    ProgramResult,
};

use crate::{
    instructions::InstructionTag,
    pack::{
        write_bytes,
        UNINIT_BYTE,
    },
    state::sector::SectorIndex,
};

/// User deposits tokens and updates or registers their seat.
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
///   2. `[WRITE]` User token account (source)
///   3. `[WRITE]` Market token account (destination)
///   4. `[READ]` Mint account
pub struct Deposit<'a> {
    /// The user depositing or registering their seat.
    pub user: &'a AccountInfo,
    /// The market account PDA.
    pub market_account: &'a AccountInfo,
    /// The user's associated token account.
    pub user_ata: &'a AccountInfo,
    /// The market's associated token account.
    pub market_ata: &'a AccountInfo,
    /// The token mint account.
    pub mint: &'a AccountInfo,
    /// The amount to deposit.
    pub amount: u64,
    /// A hint indicating which sector index the user's seat is at in the sectors array.
    /// If `NIL`, it's treated as no hint (and thus the user is registering).
    /// This avoids the need for a custom COption type.
    pub sector_index_hint: SectorIndex,
}

impl Deposit<'_> {
    #[inline(always)]
    pub fn invoke(&self) -> ProgramResult {
        self.invoke_signed(&[])
    }

    #[inline(always)]
    pub fn invoke_signed(&self, signers_seeds: &[Signer]) -> ProgramResult {
        pinocchio::cpi::invoke_signed(
            &Instruction {
                program_id: &crate::program::ID,
                accounts: &self.create_account_metas(),
                data: &self.pack_instruction_data(),
            },
            &[
                self.user,
                self.market_account,
                self.user_ata,
                self.market_ata,
                self.mint,
            ],
            signers_seeds,
        )
    }

    #[inline(always)]
    pub fn create_account_metas(&self) -> [AccountMeta; 5] {
        [
            AccountMeta::readonly_signer(self.user.key()),
            AccountMeta::writable(self.market_account.key()),
            AccountMeta::writable(self.user_ata.key()),
            AccountMeta::writable(self.market_ata.key()),
            AccountMeta::readonly(self.mint.key()),
        ]
    }

    #[inline(always)]
    pub fn pack_instruction_data(&self) -> [u8; 13] {
        // Instruction data layout:
        //   - [0]: the instruction tag, 1 byte
        //   - [1..9]: the amount as u64 little-endian bytes, 8 bytes
        //   - [9..13]: the u32 `sector_index_hint` as little-endian bytes, 4 bytes
        let mut data = [UNINIT_BYTE; 13];

        data[0].write(InstructionTag::Deposit as u8);
        write_bytes(&mut data[1..9], &self.amount.to_le_bytes());
        write_bytes(&mut data[9..13], &self.sector_index_hint.to_le_bytes());

        // Safety: All 13 bytes were written to.
        unsafe { *(data.as_ptr() as *const _) }
    }
}
