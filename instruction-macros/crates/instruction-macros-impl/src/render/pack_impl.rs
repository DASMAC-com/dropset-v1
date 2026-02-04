//! Renders the implementation for the [`crate::pack::Pack`] trait for a `derive(Pack)` struct.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    parse::parsed_struct::ParsedStruct,
    render::pack_struct_fields::{
        fully_qualified_pack_trait,
        PackStructFields,
    },
};

pub fn render(parsed_struct: ParsedStruct) -> TokenStream {
    let ParsedStruct {
        struct_ident,
        field_names,
        ..
    } = &parsed_struct;

    let PackStructFields {
        field_lengths,
        field_offsets,
    } = PackStructFields::new(&parsed_struct);

    let pack_trait = fully_qualified_pack_trait();

    // Account for structs with no fields.
    if field_names.is_empty() {
        return quote! {
            unsafe impl #pack_trait for #struct_ident {
                type Packed = [u8; 0];

                #[inline(always)]
                unsafe fn write_bytes(&self, dst: *mut u8) {}

                #[inline(always)]
                fn pack(&self) -> [u8; 0] { [] }
            }
        };
    }

    quote! {
        unsafe impl #pack_trait for #struct_ident {
            type Packed = [u8; #(#field_lengths)+*];

            #[inline(always)]
            unsafe fn write_bytes(&self, dst: *mut u8) {
                #(#pack_trait::write_bytes(&self.#field_names, dst.add(#field_offsets));)*
            }

            #[inline(always)]
            fn pack(&self) -> [u8; Self::LEN] {
                let mut buf = [::core::mem::MaybeUninit::<u8>::uninit(); Self::LEN];
                unsafe { self.write_bytes(buf.as_mut_ptr() as *mut u8) };
                unsafe { *(buf.as_ptr() as *const Self::Packed) }
            }
        }

        // Const assertion checking that `#struct_ident::LEN` is the sum of all field `::LEN`s.
        const _: [(); <#struct_ident as #pack_trait>::LEN] = [(); #(#field_lengths)+*];
    }
}
