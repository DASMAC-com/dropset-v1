use std::{
    cell::RefCell,
    rc::Rc,
};

use manifest::{
    program::{
        get_dynamic_value,
        global_create_instruction::create_global_instruction,
    },
    state::{
        GlobalFixed,
        GlobalValue,
    },
    validation::get_global_address,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::Account,
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
};

use crate::send_tx_with_retry;
#[derive(Clone)]
pub struct GlobalFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub mint_key: Pubkey,
    pub global: GlobalValue,
}

impl GlobalFixture {
    pub async fn new_with_token_program(
        context: Rc<RefCell<ProgramTestContext>>,
        mint: &Pubkey,
        token_program: &Pubkey,
    ) -> Self {
        let (global_key, _global_bump) = get_global_address(mint);
        let payer: Pubkey = context.borrow().payer.pubkey();
        let payer_keypair: Keypair = context.borrow().payer.insecure_clone();

        let context_ref: Rc<RefCell<ProgramTestContext>> = Rc::clone(&context);

        let create_global_ix: Instruction = create_global_instruction(mint, &payer, token_program);

        send_tx_with_retry(
            Rc::clone(&context),
            &[create_global_ix],
            Some(&payer),
            &[&payer_keypair, &payer_keypair],
        )
        .await
        .unwrap();

        GlobalFixture {
            context: context_ref,
            key: global_key,
            mint_key: *mint,
            global: GlobalValue {
                fixed: GlobalFixed::new_empty(mint),
                dynamic: Vec::new(),
            },
        }
    }

    pub async fn new(context: Rc<RefCell<ProgramTestContext>>, mint: &Pubkey) -> Self {
        GlobalFixture::new_with_token_program(context, mint, &spl_token::id()).await
    }

    pub async fn reload(&mut self) {
        let global_account: Account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        let global: GlobalValue = get_dynamic_value(global_account.data.as_slice());
        self.global = global;
    }
}
