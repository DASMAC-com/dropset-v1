use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_accounts(_input: DeriveInput) -> syn::Result<TokenStream> {
    Ok(TokenStream::new())
}
