//! Token-level context for creating mints, ATAs, and performing common token operations in
//! tests and examples.

use std::{
    cell::RefCell,
    collections::HashMap,
};

use solana_sdk::{
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{
        Keypair,
        Signature,
    },
    signer::Signer,
};
use spl_associated_token_account_interface::{
    address::get_associated_token_address,
    instruction::create_associated_token_account_idempotent,
};
use spl_token_2022_interface::instruction::mint_to_checked;
use spl_token_interface::state::{
    Account,
    Mint,
};

use crate::transactions::CustomRpcClient;

pub struct TokenContext {
    pub mint_authority: Keypair,
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub mint_decimals: u8,
    pub memoized_atas: RefCell<HashMap<Pubkey, Pubkey>>,
}

impl TokenContext {
    /// Creates an account, airdrops it SOL, and then uses it to create the new token mint.
    pub async fn new_token(
        rpc: &CustomRpcClient,
        token_program: Option<Pubkey>,
    ) -> anyhow::Result<Self> {
        let authority = rpc.fund_new_account().await?;
        let token_program = token_program.unwrap_or(spl_token_interface::ID);
        Self::new_token_from_mint(rpc, authority, Keypair::new(), 10, token_program).await
    }

    pub async fn new_token_from_mint(
        rpc: &CustomRpcClient,
        mint_authority: Keypair,
        mint: Keypair,
        decimals: u8,
        token_program: Pubkey,
    ) -> anyhow::Result<Self> {
        let mint_rent = rpc
            .client
            .get_minimum_balance_for_rent_exemption(Mint::LEN)?;
        let create_mint_account = solana_system_interface::instruction::create_account(
            &mint_authority.pubkey(),
            &mint.pubkey(),
            mint_rent,
            Mint::LEN as u64,
            &token_program,
        );

        let initialize_mint = spl_token_2022_interface::instruction::initialize_mint2(
            &token_program,
            &mint.pubkey(),
            &mint_authority.pubkey(),
            None,
            decimals,
        )?;

        rpc.send_and_confirm_txn(
            &mint_authority,
            &[&mint],
            &[create_mint_account, initialize_mint],
        )
        .await?;

        Ok(Self {
            mint_authority,
            mint: mint.pubkey(),
            token_program,
            mint_decimals: decimals,
            memoized_atas: RefCell::new(HashMap::new()),
        })
    }

    pub async fn create_ata_for(
        &self,
        rpc: &CustomRpcClient,
        owner: &Keypair,
    ) -> anyhow::Result<Pubkey> {
        let owner_pk = &owner.pubkey();
        let create_ata_instruction = create_associated_token_account_idempotent(
            owner_pk,
            owner_pk,
            &self.mint,
            &self.token_program,
        );
        rpc.send_and_confirm_txn(owner, &[owner], &[create_ata_instruction])
            .await?;

        Ok(self.get_ata_for(&owner.pubkey()))
    }

    pub fn get_ata_for(&self, owner: &Pubkey) -> Pubkey {
        if let Some(ata) = self.memoized_atas.borrow().get(owner) {
            return *ata;
        };

        let ata = get_associated_token_address(owner, &self.mint);
        self.memoized_atas.borrow_mut().insert(*owner, ata);

        ata
    }

    pub async fn mint_to(
        &self,
        rpc: &CustomRpcClient,
        owner: &Keypair,
        amount: u64,
    ) -> anyhow::Result<Signature> {
        let token_account = self.get_ata_for(&owner.pubkey());
        let mint_to = mint_to_checked(
            &self.token_program,
            &self.mint,
            &token_account,
            &self.mint_authority.pubkey(),
            &[],
            amount,
            self.mint_decimals,
        )?;
        rpc.send_and_confirm_txn(owner, &[&self.mint_authority], &[mint_to])
            .await
    }

    pub fn get_balance_for(&self, rpc: &CustomRpcClient, owner: &Pubkey) -> anyhow::Result<u64> {
        let ata = self.get_ata_for(owner);
        let account_data = rpc.client.get_account_data(&ata)?;
        let account_data = Account::unpack(&account_data)?;
        Ok(account_data.amount)
    }
}
