use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use transaction_parser::views::{
    MarketSeatView,
    MarketViewAll,
};

use crate::{
    context::market::MarketContext,
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
/// - Funds the default payer if it doesn't exist yet.
/// - Creates and registers a new market backed by two newly-created SPL token mints (base/quote).
/// - Airdrops [`crate::transactions::DEFAULT_FUND_AMOUNT`] lamports to each trader. If any trader
///   account already exists on-chain, returns an error.
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

    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}

impl E2e {
    pub async fn new_traders_and_market(
        rpc: Option<CustomRpcClient>,
        traders: impl AsRef<[Trader<'_>]>,
    ) -> anyhow::Result<Self> {
        let rpc = rpc.unwrap_or_default();

        // Fund the default payer if it doesn't exist yet. This is a separate account to avoid the
        // traders incurring unexpected balance changes when paying for gas.
        let default_payer = test_accounts::default_payer().insecure_clone();
        if !account_exists(&rpc.client, &default_payer.pubkey()).await? {
            rpc.fund_account(&default_payer.pubkey()).await?;
        }

        // Create and register the market derived from the base/quote token pair.
        let market = MarketContext::new_market(&rpc).await?;
        let register_market_txn = market
            .register_market(default_payer.pubkey(), 10)
            .send_single_signer(&rpc, &default_payer)
            .await?;

        // Fund and create the trader accounts if they don't exist, create their base/quote
        // associated token accounts, and mint + deposit the specified base/quote amounts to each
        // trader if the amount != 0.
        for trader in traders.as_ref().iter() {
            if !account_exists(&rpc.client, &trader.pubkey()).await? {
                rpc.fund_account(&trader.pubkey()).await?;
            }

            market.base.create_ata_for(&rpc, trader.keypair).await?;
            market.quote.create_ata_for(&rpc, trader.keypair).await?;

            if trader.base != 0 {
                market
                    .base
                    .mint_to(&rpc, trader.keypair, trader.base)
                    .await?;
            }
            if trader.quote != 0 {
                market
                    .quote
                    .mint_to(&rpc, trader.keypair, trader.quote)
                    .await?;
            }
        }

        Ok(Self {
            rpc,
            market,
            register_market_txn,
        })
    }

    pub fn view_market(&self) -> anyhow::Result<MarketViewAll> {
        self.market.view_market(&self.rpc)
    }

    pub fn find_seat(&self, user: &Pubkey) -> anyhow::Result<Option<MarketSeatView>> {
        self.market.find_seat(&self.rpc, user)
    }

    pub fn get_base_balance(&self, user: &Pubkey) -> anyhow::Result<u64> {
        self.market.base.get_balance_for(&self.rpc, user)
    }

    pub fn get_quote_balance(&self, user: &Pubkey) -> anyhow::Result<u64> {
        self.market.quote.get_balance_for(&self.rpc, user)
    }
}
