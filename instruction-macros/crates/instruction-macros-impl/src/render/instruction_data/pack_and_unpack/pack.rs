use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

/// Render the `pack` statement for an instruction data variant.
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
        0 => quote! { unsafe { #(#pack_statements)* } },
        _ => quote! {},
    };

    quote! {
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[inline(always)]
        pub fn pack(&self) -> [u8; #size_with_tag] {
            let mut data: [core::mem::MaybeUninit<u8>; #size_with_tag] = [core::mem::MaybeUninit::uninit(); #size_with_tag];
            data[0].write(super::#enum_ident::#tag_variant as u8);
            #pack_statements_tokens

            // All bytes initialized during the construction above.
            unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
        }
    }
}
