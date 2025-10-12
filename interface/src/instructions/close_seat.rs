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
/// # Safety
///
/// Caller guarantees:
/// - WRITE accounts are not currently borrowed in *any* capacity.
/// - READ accounts are not currently mutably borrowed.
///
/// ### Account
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
    pub user_base_ata: &'a AccountInfo,
    /// The user's associated quote mint token account.
    pub user_quote_ata: &'a AccountInfo,
    /// The market's associated base mint token account.
    pub market_base_ata: &'a AccountInfo,
    /// The market's associated quote mint token account.
    pub market_quote_ata: &'a AccountInfo,
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
                self.user_base_ata,
                self.user_quote_ata,
                self.market_base_ata,
                self.market_quote_ata,
                self.base_mint,
                self.quote_mint,
            ],
            signers_seeds,
        )
    }

    #[inline(always)]
    fn create_account_metas(&self) -> [AccountMeta; 8] {
        [
            AccountMeta::readonly_signer(self.user.key()),
            AccountMeta::writable(self.market_account.key()),
            AccountMeta::writable(self.user_base_ata.key()),
            AccountMeta::writable(self.user_quote_ata.key()),
            AccountMeta::writable(self.market_base_ata.key()),
            AccountMeta::writable(self.market_quote_ata.key()),
            AccountMeta::readonly(self.base_mint.key()),
            AccountMeta::readonly(self.quote_mint.key()),
        ]
    }

    #[inline(always)]
    fn pack_instruction_data(&self) -> [u8; 9] {
        let mut data = [UNINIT_BYTE; 9];
        data[0].write(InstructionTag::CloseSeat as u8);
        write_bytes(&mut data[1..9], &self.sector_index_hint.0.to_le_bytes());
        // Safety: All 9 bytes were written to.
        unsafe { *(data.as_ptr() as *const _) }
    }
}
