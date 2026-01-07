use anyhow::Context;
use dropset_interface::state::sector::SectorIndex;
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
        DEFAULT_FUND_AMOUNT,
    },
};

pub mod test_accounts;

pub struct E2e {
    pub rpc: CustomRpcClient,
    pub market: MarketContext,
    pub register_market_txn: ParsedTransactionWithEvents,
}

pub struct Trader<'a> {
    pub base: u64,
    pub quote: u64,
    pub keypair: &'a Keypair,
    pubkey: Pubkey,
}

impl<'a> Trader<'a> {
    pub fn new(keypair: &'a Keypair, base: u64, quote: u64) -> Self {
        let pubkey = keypair.pubkey();
        Self {
            base,
            quote,
            keypair,
            pubkey,
        }
    }

    pub fn pubkey(&self) -> Pubkey {
        self.pubkey
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
            rpc.fund_account(&trader.pubkey).await?;
            // Fail if any of the traders already exist, as this can cause unexpected balances.
            let trader_balance = rpc
                .client
                .get_balance(&trader.pubkey)
                .context("Couldn't retrieve the trader balance")?;
            if trader_balance != DEFAULT_FUND_AMOUNT {
                return Err(anyhow::Error::msg(format!(
                    "Trader {}'s balance {} doesn't match the default fund amount: {}",
                    trader.pubkey, trader_balance, DEFAULT_FUND_AMOUNT
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
        let register_instruction = market.register_market(default_payer.pubkey(), 10);
        let register_market_txn = rpc
            .send_single_signer(&default_payer, [register_instruction])
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

    pub async fn send_close_seat(
        &self,
        trader: &Keypair,
        seat: SectorIndex,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let close_seat = self.market.close_seat(trader.pubkey(), seat);
        self.rpc.send_single_signer(trader, [close_seat]).await
    }

    pub async fn send_deposit_base(
        &self,
        trader: &Keypair,
        amount: u64,
        seat: SectorIndex,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let deposit = self.market.deposit_base(trader.pubkey(), amount, seat);
        self.rpc.send_single_signer(trader, [deposit]).await
    }

    pub async fn send_deposit_quote(
        &self,
        trader: &Keypair,
        amount: u64,
        seat: SectorIndex,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let deposit = self.market.deposit_quote(trader.pubkey(), amount, seat);
        self.rpc.send_single_signer(trader, [deposit]).await
    }

    pub async fn send_withdraw_base(
        &self,
        trader: &Keypair,
        amount: u64,
        seat: SectorIndex,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let deposit = self.market.withdraw_base(trader.pubkey(), amount, seat);
        self.rpc.send_single_signer(trader, [deposit]).await
    }

    pub async fn send_withdraw_quote(
        &self,
        trader: &Keypair,
        amount: u64,
        seat: SectorIndex,
    ) -> anyhow::Result<ParsedTransactionWithEvents> {
        let deposit = self.market.withdraw_quote(trader.pubkey(), amount, seat);
        self.rpc.send_single_signer(trader, [deposit]).await
    }
}
