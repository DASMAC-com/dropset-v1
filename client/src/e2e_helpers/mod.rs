use solana_commitment_config::CommitmentConfig;
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
        CustomRpcClient,
        ParsedTransactionWithEvents,
    },
};

pub mod test_accounts;

/// Convenience harness for end-to-end tests and examples.
///
/// Upon instantiation it:
/// 1. Creates and registers a new market backed by two newly-created SPL token mints (base/quote).
/// 2. Airdrops [`crate::transactions::DEFAULT_FUND_AMOUNT`] to each trader. If any trader account
///    already exists on-chain, returns an error.
/// 3. Creates base/quote associated token accounts (ATAs) for each trader.
/// 4. Mints the specified `base` and `quote` amounts to each trader's ATAs.
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
        let market = MarketContext::new_market(&rpc).await?;

        // Fund the default payer.
        // This is a separate account to avoid the traders incurring unexpected balance changes when
        // paying for gas for transactions.
        let default_payer = test_accounts::default_payer().insecure_clone();
        rpc.fund_account(&default_payer.pubkey()).await?;

        // Then fund and create the trader accounts and their base/quote associated token accounts.
        // Mint and deposit the specified base/quote amounts to each trader.
        for trader in traders.as_ref().iter() {
            rpc.fund_account(&trader.pubkey()).await?;

            let account = rpc
                .client
                .get_account_with_commitment(&trader.pubkey(), CommitmentConfig::confirmed());
            let account_exists = account.is_ok_and(|v| v.value.is_some());
            if account_exists {
                // Fail if any of the traders already exist, as this can cause unexpected behavior.
                return Err(anyhow::Error::msg(format!(
                    "Trader account {} already exists.",
                    trader.pubkey(),
                )));
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

        // Then register the market.
        let register_market_txn = market
            .register_market(default_payer.pubkey(), 10)
            .send_single_signer(&rpc, &default_payer)
            .await?;

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
}
