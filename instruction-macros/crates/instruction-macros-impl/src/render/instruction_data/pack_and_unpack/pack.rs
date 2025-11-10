//! Renders the `pack` function code that serializes instruction arguments into their on-chain
//! binary layout.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

/// Render the `pack` function for an instruction data variant.
///
/// `pack` serializes instruction arguments into their on-chain binary layout.
pub fn render(
    enum_ident: &Ident,
    tag_variant: &Ident,
    layout_docs: Vec<TokenStream>,
    pack_statements: Vec<TokenStream>,
    size_with_tag: Literal,
) -> TokenStream {
    let discriminant_description =
        format!(" - [0]: the discriminant `{enum_ident}::{tag_variant}` (u8, 1 byte)");

    let pack_statements_tokens = match pack_statements.len() {
        0 => quote! {},
        _ => quote! { unsafe { #(#pack_statements)* } },
    };

    quote! {
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[inline(always)]
        pub fn pack(&self) -> [u8; #size_with_tag] {
            use ::core::mem::MaybeUninit;
            let mut data: [MaybeUninit<u8>; #size_with_tag] = [MaybeUninit::uninit(); #size_with_tag];
            data[0].write(super::#enum_ident::#tag_variant as u8);
            #pack_statements_tokens

            // All bytes initialized during the construction above.
            unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
        }
    }
}
