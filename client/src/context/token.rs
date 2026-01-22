//! Token-level context for creating mints, ATAs, and performing common token operations in
//! tests and examples.

use std::{
    collections::HashMap,
    sync::RwLock,
};

use anyhow::Ok;
use solana_address::Address;
use solana_sdk::{
    program_pack::Pack,
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
use spl_token_2022_interface::{
    check_spl_token_program_account,
    instruction::mint_to_checked,
};
use spl_token_interface::state::{
    Account,
    Mint,
};

use crate::transactions::CustomRpcClient;

pub struct TokenContext {
    /// If the mint authority is provided, [`TokenContext`] enables minting tokens directly
    /// to recipients, mostly for testing purposes.
    mint_authority: Option<Keypair>,
    pub mint_address: Address,
    pub token_program: Address,
    pub mint_decimals: u8,
    pub memoized_atas: RwLock<HashMap<Address, Address>>,
}

impl TokenContext {
    /// Creates a new [`TokenContext`] from an existing token. Checks that the token mint exists
    /// on-chain and is owned by a valid token program.
    pub fn new_from_existing(
        rpc: &CustomRpcClient,
        mint_token: Address,
        mint_authority: Option<Keypair>,
    ) -> anyhow::Result<Self> {
        let mint_account = rpc.client.get_account(&mint_token)?;
        check_spl_token_program_account(&mint_account.owner)?;
        let mint = Mint::unpack(&mint_account.data)?;

        let auth_1 = mint_authority.as_ref().map(|kp| kp.pubkey());
        let auth_2 = mint.mint_authority.into();
        // If the mint authority is passed in, ensure it matches the mint authority pubkey on-chain.
        if auth_1.is_some() && auth_1 != auth_2 {
            anyhow::bail!(
                "Mint authority passed in {auth_1:#?} doesn't match authority on-chain {auth_2:#?}"
            );
        }

        Ok(Self {
            mint_authority,
            mint_address: mint_token,
            token_program: mint_account.owner,
            mint_decimals: mint.decimals,
            memoized_atas: Default::default(),
        })
    }

    /// Creates an account, airdrops it SOL, and then uses it to create a new, random token mint.
    pub async fn create_new(
        rpc: &CustomRpcClient,
        token_program: Option<Address>,
    ) -> anyhow::Result<Self> {
        let authority = rpc.fund_new_account().await?;
        let token_program = token_program.unwrap_or(spl_token_interface::ID);
        Self::create_new_from_mint(rpc, authority, Keypair::new(), 10, token_program).await
    }

    pub async fn create_new_from_mint(
        rpc: &CustomRpcClient,
        mint_authority: Keypair,
        mint: Keypair,
        decimals: u8,
        token_program: Address,
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
            mint_authority: Some(mint_authority),
            mint_address: mint.pubkey(),
            token_program,
            mint_decimals: decimals,
            memoized_atas: RwLock::new(HashMap::new()),
        })
    }

    pub fn mint_authority(&self) -> anyhow::Result<&Keypair> {
        if self.mint_authority.is_some() {
            Ok(self.mint_authority.as_ref().unwrap())
        } else {
            anyhow::bail!("Mint authority wasn't passed to the token context")
        }
    }

    pub async fn create_ata_for(
        &self,
        rpc: &CustomRpcClient,
        owner: &Keypair,
    ) -> anyhow::Result<Address> {
        let owner_pk = &owner.pubkey();
        let create_ata_instruction = create_associated_token_account_idempotent(
            owner_pk,
            owner_pk,
            &self.mint_address,
            &self.token_program,
        );
        rpc.send_and_confirm_txn(owner, &[owner], &[create_ata_instruction])
            .await?;

        Ok(self.get_ata_for(&owner.pubkey()))
    }

    pub fn get_ata_for(&self, owner: &Address) -> Address {
        if let Some(ata) = self
            .memoized_atas
            .read()
            .expect("RWLock shouldn't be poisoned")
            .get(owner)
        {
            return *ata;
        };

        let ata = get_associated_token_address(owner, &self.mint_address);
        self.memoized_atas
            .write()
            .expect("RWLock shouldn't be poisoned")
            .insert(*owner, ata);

        ata
    }

    /// If the mint authority was passed to the token context upon creation, this mints tokens
    /// directly to the specified account. Otherwise, it fails immediately.
    pub async fn mint_to(
        &self,
        rpc: &CustomRpcClient,
        owner: &Keypair,
        amount: u64,
    ) -> anyhow::Result<Signature> {
        let mint_authority = self.mint_authority()?;
        let token_account = self.get_ata_for(&owner.pubkey());
        let mint_to = mint_to_checked(
            &self.token_program,
            &self.mint_address,
            &token_account,
            &mint_authority.pubkey(),
            &[],
            amount,
            self.mint_decimals,
        )?;
        rpc.send_and_confirm_txn(owner, &[mint_authority], &[mint_to])
            .await
            .map(|txn| txn.parsed_transaction.signature)
    }

    pub fn get_balance_for(&self, rpc: &CustomRpcClient, owner: &Address) -> anyhow::Result<u64> {
        let ata = self.get_ata_for(owner);
        let account_data = rpc.client.get_account_data(&ata)?;
        let account_data = Account::unpack(&account_data)?;
        Ok(account_data.amount)
    }
}
