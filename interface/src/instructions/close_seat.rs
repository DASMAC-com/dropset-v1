use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction, Signer},
    ProgramResult,
};

use crate::{
    instructions::InstructionTag,
    pack::{write_bytes, UNINIT_BYTE},
    state::sector::SectorIndex,
};

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
                accounts: &self.create_account_metas(),
                data: &self.pack_instruction_data(),
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

    #[inline(always)]
    pub fn create_account_metas(&self) -> [AccountMeta; 8] {
        [
            AccountMeta::readonly_signer(self.user.key()),
            AccountMeta::writable(self.market_account.key()),
            AccountMeta::writable(self.base_user_ata.key()),
            AccountMeta::writable(self.quote_user_ata.key()),
            AccountMeta::writable(self.base_market_ata.key()),
            AccountMeta::writable(self.quote_market_ata.key()),
            AccountMeta::readonly(self.base_mint.key()),
            AccountMeta::readonly(self.quote_mint.key()),
        ]
    }

    #[inline(always)]
    pub fn pack_instruction_data(&self) -> [u8; 5] {
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
