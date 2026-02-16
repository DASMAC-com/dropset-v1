use std::{
    cell::{
        Ref,
        RefCell,
    },
    rc::Rc,
};

use solana_program::{
    pubkey::Pubkey,
    rent::Rent,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::Account,
    instruction::Instruction,
    program_pack::Pack,
    signature::Keypair,
    signer::Signer,
    system_instruction::create_account,
};

use crate::send_tx_with_retry;

#[derive(Clone)]
pub struct MintFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub mint: spl_token::state::Mint,
}

impl MintFixture {
    pub async fn new(
        context: Rc<RefCell<ProgramTestContext>>,
        mint_decimals_opt: Option<u8>,
    ) -> MintFixture {
        let context_ref: Rc<RefCell<ProgramTestContext>> = Rc::clone(&context);
        let mint_keypair: Keypair = Keypair::new();
        let mint: spl_token::state::Mint = {
            let payer: Keypair = context.borrow().payer.insecure_clone();
            let rent: Rent = context.borrow_mut().banks_client.get_rent().await.unwrap();

            let init_account_ix: Instruction = create_account(
                &payer.pubkey(),
                &mint_keypair.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            );
            let init_mint_ix: Instruction = spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_keypair.pubkey(),
                &payer.pubkey(),
                None,
                mint_decimals_opt.unwrap_or(6),
            )
            .unwrap();

            send_tx_with_retry(
                Rc::clone(&context),
                &[init_account_ix, init_mint_ix],
                Some(&payer.pubkey()),
                &[&payer, &mint_keypair],
            )
            .await
            .unwrap();

            let mint_account: Account = context
                .borrow_mut()
                .banks_client
                .get_account(mint_keypair.pubkey())
                .await
                .unwrap()
                .unwrap();

            spl_token::state::Mint::unpack_unchecked(mint_account.data.as_slice()).unwrap()
        };

        MintFixture {
            context: context_ref,
            key: mint_keypair.pubkey(),
            mint,
        }
    }

    pub async fn reload(&mut self) {
        let mint_account: Account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        self.mint = spl_token::state::Mint::unpack_unchecked(mint_account.data.as_slice()).unwrap();
    }

    pub async fn mint_to(&mut self, dest: &Pubkey, num_atoms: u64) {
        let payer: Keypair = self.context.borrow().payer.insecure_clone();
        send_tx_with_retry(
            Rc::clone(&self.context),
            &[self.make_mint_to_ix(dest, num_atoms)],
            Some(&payer.pubkey()),
            &[&payer],
        )
        .await
        .unwrap();

        self.reload().await
    }

    fn make_mint_to_ix(&self, dest: &Pubkey, amount: u64) -> Instruction {
        let context: Ref<ProgramTestContext> = self.context.borrow();
        spl_token::instruction::mint_to(
            &spl_token::ID,
            &self.key,
            dest,
            &context.payer.pubkey(),
            &[&context.payer.pubkey()],
            amount,
        )
        .unwrap()
    }
}
