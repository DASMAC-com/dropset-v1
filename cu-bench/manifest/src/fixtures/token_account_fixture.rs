use std::{
    cell::RefCell,
    rc::Rc,
};

use solana_program::{
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    signature::Keypair,
    signer::Signer,
    system_instruction::create_account,
};

use crate::send_tx_with_retry;
pub struct TokenAccountFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
}

impl TokenAccountFixture {
    async fn create_ixs(
        rent: Rent,
        mint_pk: &Pubkey,
        payer_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
    ) -> [Instruction; 2] {
        let init_account_ix: Instruction = create_account(
            payer_pk,
            &keypair.pubkey(),
            rent.minimum_balance(spl_token::state::Account::LEN),
            spl_token::state::Account::LEN as u64,
            &spl_token::id(),
        );

        let init_token_ix: Instruction = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &keypair.pubkey(),
            mint_pk,
            owner_pk,
        )
        .unwrap();

        [init_account_ix, init_token_ix]
    }

    pub async fn new_with_keypair(
        context: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
        keypair: &Keypair,
    ) -> anyhow::Result<Self> {
        let rent: Rent = context.borrow_mut().banks_client.get_rent().await.unwrap();
        let payer: Pubkey = context.borrow().payer.pubkey();
        let payer_keypair: Keypair = context.borrow().payer.insecure_clone();
        let instructions: [Instruction; 2] =
            Self::create_ixs(rent, mint_pk, &payer, owner_pk, keypair).await;

        send_tx_with_retry(
            Rc::clone(&context),
            &instructions[..],
            Some(&payer),
            &[&payer_keypair, keypair],
        )
        .await?;

        Ok(Self {
            context: context.clone(),
            key: keypair.pubkey(),
        })
    }

    pub async fn new(
        context: Rc<RefCell<ProgramTestContext>>,
        mint_pk: &Pubkey,
        owner_pk: &Pubkey,
    ) -> anyhow::Result<TokenAccountFixture> {
        let keypair: Keypair = Keypair::new();
        TokenAccountFixture::new_with_keypair(context, mint_pk, owner_pk, &keypair).await
    }
}
