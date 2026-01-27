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

#[derive(Debug, Clone, Copy, strum_macros::Display, EnumIter, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab-case")]
pub enum Feature {
    Program,
    Client,
}

impl ToTokens for Feature {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append(Literal::string(&self.to_string()));
    }
}

impl Feature {
    pub fn account_view_lifetime(&self) -> TokenStream {
        match self {
            Feature::Program => quote! { 'a },
            Feature::Client => quote! {},
        }
    }

    pub fn lifetimed_ref(&self) -> TokenStream {
        match self {
            Feature::Program => quote! { &'a },
            Feature::Client => quote! {},
        }
    }

    /// The specific account view type path, without the lifetimed ref prefixed to it.
    pub fn account_view_type_path(&self) -> TokenStream {
        match self {
            Feature::Program => quote! { ::solana_account_view::AccountView },
            Feature::Client => quote! { ::solana_address::Address },
        }
    }
}
