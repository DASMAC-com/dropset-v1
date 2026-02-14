use std::{
    cell::RefCell,
    rc::Rc,
};

use manifest::{
    program::{
        create_market_instructions,
        get_dynamic_value,
    },
    state::{
        MarketFixed,
        MarketValue,
    },
    validation::MintAccountInfo,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    account::Account,
    account_info::AccountInfo,
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
};
use spl_token_2022::state::Mint;

use crate::send_tx_with_retry;
#[derive(Clone)]
pub struct MarketFixture {
    pub context: Rc<RefCell<ProgramTestContext>>,
    pub key: Pubkey,
    pub market: MarketValue,
}

impl MarketFixture {
    pub async fn new(
        context: Rc<RefCell<ProgramTestContext>>,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
    ) -> Self {
        let market_keypair: Keypair = Keypair::new();
        let payer: Pubkey = context.borrow().payer.pubkey();
        let payer_keypair: Keypair = context.borrow().payer.insecure_clone();
        let create_market_ixs: Vec<Instruction> =
            create_market_instructions(&market_keypair.pubkey(), base_mint, quote_mint, &payer)
                .unwrap();

        send_tx_with_retry(
            Rc::clone(&context),
            &create_market_ixs[..],
            Some(&payer),
            &[&payer_keypair, &market_keypair],
        )
        .await
        .unwrap();

        let context_ref: Rc<RefCell<ProgramTestContext>> = Rc::clone(&context);

        // Dummy MintAccountInfo values — only used to create an empty MarketFixed
        // placeholder until reload() populates the real data.
        let mut lamports: u64 = 0;
        let base_mint_info: MintAccountInfo = MintAccountInfo {
            mint: Mint {
                mint_authority: None.into(),
                supply: 0,
                decimals: 6,
                is_initialized: true,
                freeze_authority: None.into(),
            },
            info: &AccountInfo {
                key: &Pubkey::new_unique(),
                lamports: Rc::new(RefCell::new(&mut lamports)),
                data: Rc::new(RefCell::new(&mut [])),
                owner: &Pubkey::new_unique(),
                rent_epoch: 0,
                is_signer: false,
                is_writable: false,
                executable: false,
            },
        };

        let mut lamports: u64 = 0;
        let quote_mint_info: MintAccountInfo = MintAccountInfo {
            mint: Mint {
                mint_authority: None.into(),
                supply: 0,
                decimals: 9,
                is_initialized: true,
                freeze_authority: None.into(),
            },
            info: &AccountInfo {
                key: &Pubkey::new_unique(),
                lamports: Rc::new(RefCell::new(&mut lamports)),
                data: Rc::new(RefCell::new(&mut [])),
                owner: &Pubkey::new_unique(),
                rent_epoch: 0,
                is_signer: false,
                is_writable: false,
                executable: false,
            },
        };

        MarketFixture {
            context: context_ref,
            key: market_keypair.pubkey(),
            market: MarketValue {
                fixed: MarketFixed::new_empty(
                    &base_mint_info,
                    &quote_mint_info,
                    &market_keypair.pubkey(),
                ),
                dynamic: Vec::new(),
            },
        }
    }

    pub async fn reload(&mut self) {
        let market_account: Account = self
            .context
            .borrow_mut()
            .banks_client
            .get_account(self.key)
            .await
            .unwrap()
            .unwrap();

        let market: MarketValue = get_dynamic_value(market_account.data.as_slice());
        self.market = market;
    }
}
