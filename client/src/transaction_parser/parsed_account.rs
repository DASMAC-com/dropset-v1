use derive_more::{
    AsRef,
    Deref,
    Index,
    IntoIterator,
};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status_client_types::ParsedAccount as SdkParsedAccount;

#[derive(Copy, Clone, Debug)]
pub struct ParsedAccount {
    pub pubkey: Pubkey,
    pub writable: bool,
    pub signer: bool,
}

impl From<&ParsedAccount> for Pubkey {
    fn from(account: &ParsedAccount) -> Self {
        account.pubkey
    }
}

impl From<SdkParsedAccount> for ParsedAccount {
    fn from(account: SdkParsedAccount) -> Self {
        Self {
            pubkey: Pubkey::from_str_const(&account.pubkey),
            writable: account.writable,
            signer: account.signer,
        }
    }
}

#[derive(Clone, Debug, Default, Deref, Index, IntoIterator, AsRef)]
pub struct ParsedAccounts(Vec<ParsedAccount>);

impl ParsedAccounts {
    pub fn pubkeys(&self) -> Vec<Pubkey> {
        self.iter().map(|p| p.pubkey).collect()
    }
}

impl FromIterator<ParsedAccount> for ParsedAccounts {
    fn from_iter<I: IntoIterator<Item = ParsedAccount>>(iter: I) -> Self {
        ParsedAccounts(iter.into_iter().collect())
    }
}
