//! Renders the `pack` function code that serializes instruction arguments into their on-chain
//! binary layout.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::{
    parse::argument_type::Size,
    render::pack_struct_fields::{
        fully_qualified_pack_trait,
        fully_qualified_tagged_trait,
    },
};

pub struct Packs {
    pub pack_tagged_fn: TokenStream,
    pub write_bytes_tagged_fn: TokenStream,
}

/// Render the `pack_tagged` function for an instruction data variant.
///
/// `pack_tagged` serializes instruction arguments into their on-chain binary layout.
pub fn render(
    enum_ident: &Ident,
    size_with_tag: Size,
    tag_variant: &Ident,
    layout_docs: Vec<TokenStream>,
) -> Packs {
    let discriminant_description =
        format!(" - `[0]` **the discriminant** `{enum_ident}::{tag_variant}` (`u8`, 1 byte)");

    let safety_check_line = format!(" - `dst` has {} writable bytes.", quote! { #size_with_tag });
    let pack_trait = fully_qualified_pack_trait();
    let tagged_trait = fully_qualified_tagged_trait();

    let pack_tagged_fn = quote! {
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[inline(always)]
        pub fn pack_tagged(&self) -> [u8; #size_with_tag] {
            use ::core::mem::MaybeUninit;
            let mut data: [MaybeUninit<u8>; #size_with_tag] = [MaybeUninit::uninit(); #size_with_tag];
            let dst = data.as_mut_ptr() as *mut u8;
            // # Safety: `dst` has sufficient writable bytes.
            unsafe { <Self as #tagged_trait>::write_bytes_tagged(self, dst) };

            // All bytes initialized during the construction above.
            unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
        }
    };

    let write_bytes_tagged_fn = quote! {
        /// Writes the `Self::TAG_BYTE` to a destination pointer and *then* calls
        /// `<Self as Pack>::write_bytes` starting at the 1-byte offset of the same pointer.
        #[doc = ""]
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[doc = ""]
        /// # Safety
        ///
        /// Caller must guarantee:
        ///
        #[doc = #safety_check_line]
        #[inline(always)]
        unsafe fn write_bytes_tagged(&self, dst: *mut u8) {
            dst.write(Self::TAG_BYTE);
            <Self as #pack_trait>::write_bytes(self, dst.add(1));
        }
    };

    Packs {
        pack_tagged_fn,
        write_bytes_tagged_fn,
    }
}
