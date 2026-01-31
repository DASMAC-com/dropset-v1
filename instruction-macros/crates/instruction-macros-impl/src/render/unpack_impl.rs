//! Renders the implementation for the [`crate::unpack::Unpack`] trait for a `derive(Unpack)`
//! struct.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    parse::{
        error_path::ErrorPath,
        error_type::ErrorType,
        parsed_struct::ParsedStruct,
    },
    render::pack_struct_fields::{
        fully_qualified_pack_trait,
        fully_qualified_unpack_trait,
        PackStructFields,
    },
};

pub fn render(parsed_struct: ParsedStruct) -> TokenStream {
    let ParsedStruct {
        struct_ident,
        field_names,
        field_types,
    } = &parsed_struct;

    let field_offsets = PackStructFields::new(&parsed_struct).field_offsets;

    let ErrorPath { base, variant } = ErrorType::InvalidInstructionData.to_path();

    let pack_trait = fully_qualified_pack_trait();
    let unpack_trait = fully_qualified_unpack_trait();

    quote! {
        unsafe impl #unpack_trait for #struct_ident {
            #[inline(always)]
            unsafe fn read_bytes(src: *const u8) -> Result<Self, #base> {
                Ok(Self {
                    #(#field_names: <#field_types as #unpack_trait>::read_bytes(src.add(#field_offsets))?,)*
                })
            }

            #[inline(always)]
            fn unpack(data: &[u8]) -> Result<Self, #base> {
                if data.len() < <Self as #pack_trait>::LEN {
                    return Err(#base::#variant);
                }

                /// Safety: The length of `data` was just verified as sufficient.
                unsafe { Self::read_bytes(data.as_ptr()) }
            }
        }
    }
}
