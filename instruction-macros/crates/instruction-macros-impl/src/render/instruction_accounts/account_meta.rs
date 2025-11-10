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
    /// Generates Solana `AccountMeta` constructors for each instruction’s accounts, suitable for
    /// building instruction account lists for each of the various [`Feature`] types.
    pub fn render_account_meta(&self, feature: Feature) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        match feature {
            Feature::Pinocchio => {
                let ctor_method = match (self.is_writable, self.is_signer) {
                    (true, true) => quote! { writable_signer },
                    (true, false) => quote! { writable },
                    (false, true) => quote! { readonly_signer },
                    (false, false) => quote! { readonly },
                };
                quote! { ::pinocchio::instruction::AccountMeta::#ctor_method(self.#field_ident.key()) }
            }
            Feature::SolanaProgram => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = self.is_signer;
                quote! { ::solana_instruction::AccountMeta::#ctor_method(*self.#field_ident.key, #is_signer) }
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
