//! Token-level context containing mint metadata and helpers for deriving associated token accounts
//! and building token instructions.

use solana_address::Address;
use solana_instruction::Instruction;
use solana_sdk::program_pack::Pack;
use spl_associated_token_account_interface::{
    address::get_associated_token_address,
    instruction::create_associated_token_account_idempotent,
};
use spl_token_2022_interface::{
    check_spl_token_program_account,
    instruction::mint_to_checked,
};
use spl_token_interface::state::Mint;

pub struct TokenContext {
    pub mint_address: Address,
    pub token_program: Address,
    pub mint_decimals: u8,
}

impl TokenContext {
    pub fn new(mint_address: Address, token_program: Address, mint_decimals: u8) -> Self {
        Self {
            mint_address,
            token_program,
            mint_decimals,
        }
    }

    /// Creates a [`TokenContext`] from an on-chain mint account's owner and data.
    ///
    /// Validates that the owner is a recognized SPL token program and unpacks the mint to extract
    /// the decimals.
    pub fn from_account_data(
        mint_address: Address,
        owner: Address,
        data: &[u8],
    ) -> anyhow::Result<Self> {
        check_spl_token_program_account(&owner)?;
        let mint = Mint::unpack(data)?;
        Ok(Self::new(mint_address, owner, mint.decimals))
    }

    pub fn get_ata_for(&self, owner: &Address) -> Address {
        get_associated_token_address(owner, &self.mint_address)
    }

    /// Builds an idempotent create-ATA instruction for the given `owner`, funded by `funder`.
    pub fn create_ata_idempotent(&self, funder: &Address, owner: &Address) -> Instruction {
        create_associated_token_account_idempotent(
            funder,
            owner,
            &self.mint_address,
            &self.token_program,
        )
    }

    /// Builds a `mint_to_checked` instruction that mints `amount` tokens to `destination`.
    pub fn mint_to(
        &self,
        mint_authority: &Address,
        destination: &Address,
        amount: u64,
    ) -> anyhow::Result<Instruction> {
        Ok(mint_to_checked(
            &self.token_program,
            &self.mint_address,
            destination,
            mint_authority,
            &[],
            amount,
            self.mint_decimals,
        )?)
    }
}
