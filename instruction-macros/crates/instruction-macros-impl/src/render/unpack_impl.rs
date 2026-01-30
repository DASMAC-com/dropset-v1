//! Renders the implementation for the [`crate::unpack::Unpack`] trait for a `derive(Unpack)`
//! struct.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    parse::parsed_struct::ParsedStruct,
    render::pack_struct_fields::PackStructFields,
};

pub fn render(parsed_struct: ParsedStruct) -> TokenStream {
    let ParsedStruct {
        struct_ident,
        field_names,
        field_types,
    } = &parsed_struct;

    let field_offsets = PackStructFields::new(&parsed_struct).field_offsets;

    // Fully qualify the path, otherwise it collides with the proc macro derive with the same name.
    let unpack_trait = quote! { ::instruction_macros::Unpack };

    let struct_fields = quote! {
        #(#field_names: <#field_types as #unpack_trait>::unpack(src.add(#field_offsets))?,)*
    };

    quote! {
        unsafe impl #unpack_trait for #struct_ident {
            unsafe fn unpack(src: *const u8) -> Result<Self, ::solana_program_error::ProgramError> {
                Ok(Self {
                    #struct_fields
                })
            }
        }
    }
}
