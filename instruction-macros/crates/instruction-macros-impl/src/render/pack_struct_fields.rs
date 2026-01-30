use proc_macro2::TokenStream;
use quote::quote;

use crate::parse::parsed_struct::ParsedStruct;

pub fn fully_qualified_pack_trait() -> TokenStream {
    // Fully qualify the path, otherwise it collides with the proc macro derive with the same name.
    quote! { ::instruction_macros::Pack }
}

pub struct PackStructFields {
    pub field_lengths: Vec<TokenStream>,
    pub field_offsets: Vec<TokenStream>,
}

impl PackStructFields {
    pub fn new(parsed_struct: &ParsedStruct) -> Self {
        let field_types = &parsed_struct.field_types;
        let pack_trait = fully_qualified_pack_trait();

        let (field_lengths, field_offsets) = field_types.iter().fold(
            (vec![], vec![]),
            |(mut lengths, mut offsets), field_type| {
                // The offset is the cumulative lengths of all the fields before the current field.
                let offset = match lengths.len() {
                    0 => quote! { 0 },
                    _ => quote! { #(#lengths)+* },
                };
                let length = quote! { <#field_type as #pack_trait>::LEN };

                lengths.push(length);
                offsets.push(offset);

                (lengths, offsets)
            },
        );

        PackStructFields {
            field_lengths,
            field_offsets,
        }
    }
}
