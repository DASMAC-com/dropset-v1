//! Describes the supported codegen features/targets and provides helpers to conditionally
//! enable or disable parts of the generated API.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    quote,
    ToTokens,
    TokenStreamExt,
};
use strum_macros::EnumIter;

#[derive(Clone, Copy, strum_macros::Display, EnumIter, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab-case")]
pub enum Feature {
    SolanaProgram,
    Pinocchio,
    Client,
}

impl ToTokens for Feature {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Literal::string(&self.to_string()));
    }
}

impl Feature {
    pub fn account_info_lifetime(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { 'a, 'info },
            Feature::Pinocchio => quote! { 'a },
            Feature::Client => quote! {},
        }
    }

    pub fn lifetimed_ref(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { &'a },
            Feature::Pinocchio => quote! { &'a },
            Feature::Client => quote! {},
        }
    }

    /// The specific account info type path, without the lifetimed ref prefixed to it.
    pub fn account_info_type_path(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { ::solana_sdk::account_info::AccountInfo<'info> },
            Feature::Pinocchio => quote! { ::pinocchio::account_info::AccountInfo },
            Feature::Client => quote! { ::solana_sdk::pubkey::Pubkey },
        }
    }
}
