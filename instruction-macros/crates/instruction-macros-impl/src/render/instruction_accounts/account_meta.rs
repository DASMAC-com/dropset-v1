//! See [`InstructionAccount::render_account_meta`].

use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};

use crate::{
    parse::instruction_account::InstructionAccount,
    render::Feature,
};

impl InstructionAccount {
    /// Generates Solana `InstructionAccount` constructors for each instruction’s accounts.
    pub fn render_instruction_account_view(&self, feature: Feature) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        match feature {
            Feature::Program => {
                let ctor_method = match (self.is_writable, self.is_signer) {
                    (true, true) => quote! { writable_signer },
                    (true, false) => quote! { writable },
                    (false, true) => quote! { readonly_signer },
                    (false, false) => quote! { readonly },
                };
                quote! { ::solana_instruction_view::InstructionAccount::#ctor_method(self.#field_ident.address()) }
            }
            Feature::Client => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = format_ident!("{}", self.is_signer);
                quote! { ::solana_instruction::AccountMeta::#ctor_method(self.#field_ident, #is_signer) }
            }
        }
    }
}
