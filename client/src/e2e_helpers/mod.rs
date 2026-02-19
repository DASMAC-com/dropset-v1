use solana_address::Address;
use solana_sdk::{
    program_pack::Pack,
    signature::{
        Keypair,
        Signer,
    },
};
use spl_token_interface::state::{
    Account,
    Mint,
};
use transaction_parser::views::{
    try_market_view_all_from_owner_and_data,
    MarketSeatView,
    MarketViewAll,
};

use crate::{
    context::{
        market::MarketContext,
        token::TokenContext,
    },
    single_signer_instruction::SingleSignerInstruction,
    token_instructions::create_and_initialize_token_instructions,
    transactions::{
        account_exists,
        CustomRpcClient,
        ParsedTransactionWithEvents,
    },
};

pub mod test_accounts;

/// Convenience harness for end-to-end tests and examples.
///
/// Upon instantiation it:
/// - Airdrops [`crate::transactions::DEFAULT_FUND_AMOUNT`] lamports to the
///   [`test_accounts::default_payer`] account.
/// - Creates and registers a new market backed by two newly-created SPL token mints (base/quote).
///   The [`test_accounts::default_payer`] account is the registrant.
/// - Airdrops [`crate::transactions::DEFAULT_FUND_AMOUNT`] lamports to each trader.
/// - Creates base/quote associated token accounts (ATAs) for each trader.
/// - Mints the specified `base` and `quote` amounts to each trader's ATAs if the amount is != 0.
pub struct E2e {
    pub rpc: CustomRpcClient,
    pub market: MarketContext,
    pub register_market_txn: ParsedTransactionWithEvents,
}

/// Setup config for a trader in [`E2e::new_traders_and_market`].
///
/// Bundles a signer with initial `base` / `quote` amounts.
pub struct Trader<'a> {
    pub base: u64,
    pub quote: u64,
    pub keypair: &'a Keypair,
}

impl<'a> Trader<'a> {
    pub fn new(keypair: &'a Keypair, base: u64, quote: u64) -> Self {
        Self {
            base,
            quote,
            keypair,
        }
    }

    pub fn address(&self) -> Address {
        self.keypair.pubkey()
    }
}

impl E2e {
    pub async fn new_traders_and_market(
        rpc: Option<CustomRpcClient>,
        traders: impl AsRef<[Trader<'_>]>,
    ) -> anyhow::Result<Self> {
        let rpc = rpc.unwrap_or_default();

        let default_payer = test_accounts::default_payer().insecure_clone();
        if !account_exists(&rpc.client, &default_payer.pubkey()).await? {
            rpc.fund_account(&default_payer.pubkey()).await?;
        }

        // Create new random base/quote tokens and derive the market context from them.
        let (base, base_mint_authority) = create_token(&rpc, None).await?;
        let (quote, quote_mint_authority) = create_token(&rpc, None).await?;
        let market = MarketContext::new(base, quote);

        let register_market_txn = market
            .register_market(default_payer.pubkey(), 10)
            .send_single_signer(&rpc, &default_payer)
            .await?;

        // Fund and create the trader accounts, create their base/quote associated token accounts,
        // and mint + deposit the specified base/quote amounts to each trader if the amount
        // != 0.
        for trader in traders.as_ref().iter() {
            rpc.fund_account(&trader.address()).await?;

            create_ata(&rpc, &market.base, trader.keypair).await?;
            create_ata(&rpc, &market.quote, trader.keypair).await?;

            if trader.base != 0 {
                mint_to(
                    &rpc,
                    &market.base,
                    &base_mint_authority,
                    trader.keypair,
                    trader.base,
                )
                .await?;
            }
            if trader.quote != 0 {
                mint_to(
                    &rpc,
                    &market.quote,
                    &quote_mint_authority,
                    trader.keypair,
                    trader.quote,
                )
                .await?;
            }
        }

        Ok(Self {
            rpc,
            market,
            register_market_txn,
        })
    }

    pub async fn view_market(&self) -> anyhow::Result<MarketViewAll> {
        let market_account = self.rpc.client.get_account(&self.market.market).await?;
        try_market_view_all_from_owner_and_data(market_account.owner, &market_account.data)
    }

    pub async fn fetch_seat(&self, user: &Address) -> anyhow::Result<Option<MarketSeatView>> {
        let market = self.view_market().await?;
        Ok(self.market.find_seat(&market.seats, user))
    }

    pub fn find_seat(&self, seats: &[MarketSeatView], user: &Address) -> Option<MarketSeatView> {
        self.market.find_seat(seats, user)
    }

    pub async fn get_base_balance(&self, user: &Address) -> anyhow::Result<u64> {
        get_token_balance(&self.rpc, &self.market.base, user).await
    }

    pub async fn get_quote_balance(&self, user: &Address) -> anyhow::Result<u64> {
        get_token_balance(&self.rpc, &self.market.quote, user).await
    }
}

/// Creates a new token mint on-chain. Returns the [`TokenContext`] and the mint authority keypair.
async fn create_token(
    rpc: &CustomRpcClient,
    token_program: Option<Address>,
) -> anyhow::Result<(TokenContext, Keypair)> {
    let authority = rpc.fund_new_account().await?;
    let mint = Keypair::new();
    let token_program = token_program.unwrap_or(spl_token_interface::ID);
    let decimals = 10;

    let mint_rent = rpc
        .client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
        .await?;

    let (create_mint_account, initialize_mint) = create_and_initialize_token_instructions(
        &authority.pubkey(),
        &mint.pubkey(),
        mint_rent,
        decimals,
        &token_program,
    )?;

    rpc.send_and_confirm_txn(
        &authority,
        &[&mint],
        &[create_mint_account, initialize_mint],
    )
    .await?;

    let token = TokenContext::new(mint.pubkey(), token_program, decimals);
    Ok((token, authority))
}

/// Creates an associated token account for the owner.
async fn create_ata(
    rpc: &CustomRpcClient,
    token: &TokenContext,
    owner: &Keypair,
) -> anyhow::Result<Address> {
    let owner_pk = owner.pubkey();
    let ix = token.create_ata(&owner.pubkey(), &owner_pk);
    rpc.send_and_confirm_txn(owner, &[], &[ix]).await?;
    Ok(token.get_ata_for(&owner_pk))
}

/// Mints tokens to the owner's ATA.
async fn mint_to(
    rpc: &CustomRpcClient,
    token: &TokenContext,
    mint_authority: &Keypair,
    owner: &Keypair,
    amount: u64,
) -> anyhow::Result<()> {
    let destination = token.get_ata_for(&owner.pubkey());
    let ix = token.mint_to(&mint_authority.pubkey(), &destination, amount)?;
    rpc.send_and_confirm_txn(owner, &[mint_authority], &[ix])
        .await?;
    Ok(())
}

/// Fetches a token balance for a user.
async fn get_token_balance(
    rpc: &CustomRpcClient,
    token: &TokenContext,
    owner: &Address,
) -> anyhow::Result<u64> {
    let ata = token.get_ata_for(owner);
    let account_data = rpc.client.get_account_data(&ata).await?;
    let account_data = Account::unpack(&account_data)?;
    Ok(account_data.amount)
}
