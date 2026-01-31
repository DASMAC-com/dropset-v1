//! Renders the `pack` function code that serializes instruction arguments into their on-chain
//! binary layout.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::render::pack_struct_fields::fully_qualified_pack_trait;

pub struct Packs {
    pub pack_tagged: TokenStream,
    pub pack_into_slice: TokenStream,
}

/// Render the `pack` function for an instruction data variant.
///
/// `pack` serializes instruction arguments into their on-chain binary layout.
pub fn render(
    enum_ident: &Ident,
    struct_name: &Ident,
    tag_variant: &Ident,
    layout_docs: Vec<TokenStream>,
    pack_statements: Vec<TokenStream>,
    size_with_tag: Literal,
) -> Packs {
    let discriminant_description =
        format!(" - `[0]` **the discriminant** `{enum_ident}::{tag_variant}` (`u8`, 1 byte)");

    let pack_statements_tokens = match pack_statements.len() {
        0 => quote! {},
        _ => quote! { unsafe { #(#pack_statements)* } },
    };

    let buf_len_check_line = format!(" - `buf.len() >= offset + {}`.", size_with_tag);
    let pack_trait = fully_qualified_pack_trait();

    let pack_tagged = quote! {
        #[doc = " Instruction data layout:"]
        #[doc = #discriminant_description]
        #(#layout_docs)*
        #[inline(always)]
        pub fn pack_tagged(&self) -> [u8; #size_with_tag] {
            use ::core::mem::MaybeUninit;
            let mut data: [MaybeUninit<u8>; #size_with_tag] = [MaybeUninit::uninit(); #size_with_tag];
            let ptr = data.as_mut_ptr() as *mut u8;
            unsafe {
                ptr.write(Self::TAG_BYTE);
                <Self as #pack_trait>::write_bytes(self, ptr.add(1));
            }

            // All bytes initialized during the construction above.
            unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
        }
    };

    let pack_into_slice = quote! {
        impl PackIntoSlice for #struct_name {
            /// This is the byte length **including** the tag byte; i.e., the size of the full event
            /// instruction data in an `instruction_data: &[u8]` slice with the tag.
            const LEN_WITH_TAG: usize = #size_with_tag;

            #[doc = " Instruction data layout:"]
            #[doc = #discriminant_description]
            #(#layout_docs)*
            #[doc = ""]
            /// # Safety
            ///
            /// Caller must guarantee:
            ///
            #[doc = #buf_len_check_line]
            ///
            /// The caller is also responsible for tracking how much of `buf` is
            /// considered initialized after the call.
            #[inline(always)]
            unsafe fn pack_into_slice(&self, buf: &mut [::core::mem::MaybeUninit<u8>], offset: usize) {
                use ::core::{mem::MaybeUninit, slice};

                let ptr = buf.as_mut_ptr().add(offset);
                let data: &mut [MaybeUninit<u8>] = slice::from_raw_parts_mut(ptr, #size_with_tag);

                data[0].write(Self::TAG_BYTE);
                #pack_statements_tokens
            }
        }
    };

    Packs {
        pack_tagged,
        pack_into_slice,
    }
}
