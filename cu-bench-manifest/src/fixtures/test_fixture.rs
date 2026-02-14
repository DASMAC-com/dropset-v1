use std::{
    cell::RefCell,
    rc::Rc,
};

use manifest::{
    self,
    program::{
        batch_update::{
            CancelOrderParams,
            PlaceOrderParams,
        },
        batch_update_instruction,
        claim_seat_instruction::claim_seat_instruction,
        deposit_instruction,
    },
};
use solana_program::pubkey::Pubkey;
use solana_program_runtime::execution_budget::MAX_COMPUTE_UNIT_LIMIT;
use solana_program_test::{
    processor,
    BanksClientError,
    ProgramTest,
    ProgramTestContext,
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
};

use crate::{
    send_tx_with_retry,
    GlobalFixture,
    MarketFixture,
    MintFixture,
    Token,
    TokenAccountFixture,
};

pub struct TestFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub sol_mint_fixture: MintFixture,
    pub usdc_mint_fixture: MintFixture,
    pub payer_sol_fixture: TokenAccountFixture,
    pub payer_usdc_fixture: TokenAccountFixture,
    pub market_fixture: MarketFixture,
    pub global_fixture: GlobalFixture,
    pub sol_global_fixture: GlobalFixture,
    pub logs: String,
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        if self.logs.is_empty() {
            return;
        }
        eprintln!("\n{}", self.logs);
    }
}

impl TestFixture {
    pub async fn new() -> TestFixture {
        let program: ProgramTest = ProgramTest::new(
            "manifest",
            manifest::ID,
            processor!(manifest::process_instruction),
        );

        let context: Rc<RefCell<ProgramTestContext>> =
            Rc::new(RefCell::new(program.start_with_context().await));
        solana_logger::setup_with_default_filter();

        let usdc_mint_f: MintFixture = MintFixture::new(Rc::clone(&context), Some(6)).await;
        let sol_mint_f: MintFixture = MintFixture::new(Rc::clone(&context), Some(9)).await;
        let mut market_fixture: MarketFixture =
            MarketFixture::new(Rc::clone(&context), &sol_mint_f.key, &usdc_mint_f.key).await;

        let mut global_fixture: GlobalFixture =
            GlobalFixture::new(Rc::clone(&context), &usdc_mint_f.key).await;
        let mut sol_global_fixture: GlobalFixture =
            GlobalFixture::new(Rc::clone(&context), &sol_mint_f.key).await;

        let payer: Pubkey = context.borrow().payer.pubkey();
        let payer_sol_fixture: TokenAccountFixture =
            TokenAccountFixture::new(Rc::clone(&context), &sol_mint_f.key, &payer)
                .await
                .expect("Should create account");
        let payer_usdc_fixture =
            TokenAccountFixture::new(Rc::clone(&context), &usdc_mint_f.key, &payer)
                .await
                .expect("Should create account");
        market_fixture.reload().await;
        global_fixture.reload().await;
        sol_global_fixture.reload().await;

        TestFixture {
            context: Rc::clone(&context),
            usdc_mint_fixture: usdc_mint_f,
            sol_mint_fixture: sol_mint_f,
            market_fixture,
            global_fixture,
            sol_global_fixture,
            payer_sol_fixture,
            payer_usdc_fixture,
            logs: Default::default(),
        }
    }

    pub fn payer(&self) -> Pubkey {
        self.context.borrow().payer.pubkey()
    }

    pub fn payer_keypair(&self) -> Keypair {
        self.context.borrow().payer.insecure_clone()
    }

    pub async fn claim_seat(&self) -> anyhow::Result<(), BanksClientError> {
        self.claim_seat_for_keypair(&self.payer_keypair()).await
    }

    pub async fn claim_seat_for_keypair(
        &self,
        keypair: &Keypair,
    ) -> anyhow::Result<(), BanksClientError> {
        let claim_seat_ix: Instruction =
            claim_seat_instruction(&self.market_fixture.key, &keypair.pubkey());
        send_tx_with_retry(
            Rc::clone(&self.context),
            &[claim_seat_ix],
            Some(&keypair.pubkey()),
            &[keypair],
        )
        .await
    }

    pub async fn deposit(
        &mut self,
        token: Token,
        num_atoms: u64,
    ) -> anyhow::Result<(), BanksClientError> {
        self.deposit_for_keypair(token, num_atoms, &self.payer_keypair())
            .await?;
        Ok(())
    }

    pub async fn deposit_for_keypair(
        &mut self,
        token: Token,
        num_atoms: u64,
        keypair: &Keypair,
    ) -> anyhow::Result<(), BanksClientError> {
        let is_base: bool = token == Token::SOL;
        let (mint, trader_token_account) = if is_base {
            let trader_token_account: Pubkey = if keypair.pubkey() == self.payer() {
                self.payer_sol_fixture.key
            } else {
                let token_account_keypair: Keypair = Keypair::new();
                let token_account_fixture: TokenAccountFixture =
                    TokenAccountFixture::new_with_keypair(
                        Rc::clone(&self.context),
                        &self.sol_mint_fixture.key,
                        &keypair.pubkey(),
                        &token_account_keypair,
                    )
                    .await
                    .expect("Should create account");
                token_account_fixture.key
            };
            self.sol_mint_fixture
                .mint_to(&trader_token_account, num_atoms)
                .await;
            (&self.sol_mint_fixture.key, trader_token_account)
        } else {
            let trader_token_account: Pubkey = if keypair.pubkey() == self.payer() {
                self.payer_usdc_fixture.key
            } else {
                let token_account_keypair: Keypair = Keypair::new();
                let token_account_fixture: TokenAccountFixture =
                    TokenAccountFixture::new_with_keypair(
                        Rc::clone(&self.context),
                        &self.usdc_mint_fixture.key,
                        &keypair.pubkey(),
                        &token_account_keypair,
                    )
                    .await
                    .expect("Should create account");
                token_account_fixture.key
            };
            self.usdc_mint_fixture
                .mint_to(&trader_token_account, num_atoms)
                .await;
            (&self.usdc_mint_fixture.key, trader_token_account)
        };

        let deposit_ix: Instruction = deposit_instruction(
            &self.market_fixture.key,
            &keypair.pubkey(),
            mint,
            num_atoms,
            &trader_token_account,
            spl_token::id(),
            None,
        );

        send_tx_with_retry(
            Rc::clone(&self.context),
            &[deposit_ix],
            Some(&keypair.pubkey()),
            &[keypair],
        )
        .await
    }

    pub async fn batch_update_for_keypair(
        &mut self,
        trader_index_hint: Option<manifest::deps::hypertree::DataIndex>,
        cancels: Vec<CancelOrderParams>,
        orders: Vec<PlaceOrderParams>,
        keypair: &Keypair,
    ) -> anyhow::Result<(), BanksClientError> {
        let batch_update_ix: Instruction = batch_update_instruction(
            &self.market_fixture.key,
            &keypair.pubkey(),
            trader_index_hint,
            cancels,
            orders,
            None,
            None,
            None,
            None,
        );
        send_tx_with_retry(
            Rc::clone(&self.context),
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(MAX_COMPUTE_UNIT_LIMIT),
                ComputeBudgetInstruction::set_compute_unit_price(1),
                batch_update_ix,
            ],
            Some(&keypair.pubkey()),
            &[keypair],
        )
        .await
    }
}
