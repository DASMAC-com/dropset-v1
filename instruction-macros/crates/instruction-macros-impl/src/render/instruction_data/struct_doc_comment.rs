use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::parse::instruction_argument::InstructionArgument;

pub fn render(
    enum_ident: &Ident,
    tag_variant: &Ident,
    instruction_args: &[InstructionArgument],
) -> TokenStream {
    let first_line = format!(" `{}::{}` instruction data.", enum_ident, tag_variant);

    let remaining = instruction_args
        .iter()
        .map(|a| {
            let line = match a.description.is_empty() {
                true => format!(" - `{}`", a.name),
                false => format!(" - `{}` — {}", a.name, a.description),
            };
            quote! { #[doc = #line] }
        })
        .collect::<Vec<_>>();

    quote! {
        #[doc = #first_line]
        #(
            #[doc = ""]
            #remaining
        )*
    }
}
