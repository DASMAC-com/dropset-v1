//! Renders the `pack_tagged` code.
//!
//! See [`render`].

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{
    parse::argument_type::Size,
    render::pack_struct_fields::fully_qualified_tagged_trait,
};

/// Render the `Tagged::pack_tagged` impl for an instruction data variant.
///
/// `pack_tagged` serializes instruction arguments into their on-chain binary layout with the tag
/// byte prepended to the front of the byte array.
pub fn render(
    enum_ident: &Ident,
    size_with_tag: Size,
    tag_variant: &Ident,
    layout_docs: Vec<TokenStream>,
) -> TokenStream {
    let discriminant_description =
        format!(" - `[0]` **the discriminant** `{enum_ident}::{tag_variant}` (`u8`, 1 byte)");

    let tagged_trait = fully_qualified_tagged_trait();

    quote! {
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[inline(always)]
        fn pack_tagged(&self) -> [u8; #size_with_tag] {
            use ::core::mem::MaybeUninit;
            let mut data: [MaybeUninit<u8>; #size_with_tag] = [MaybeUninit::uninit(); #size_with_tag];
            let dst = data.as_mut_ptr() as *mut u8;
            // # Safety: `dst` has sufficient writable bytes.
            unsafe { <Self as #tagged_trait>::write_bytes_tagged(self, dst) };

            // # Safety: All bytes are initialized during the construction above.
            unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
        }
    }
}
