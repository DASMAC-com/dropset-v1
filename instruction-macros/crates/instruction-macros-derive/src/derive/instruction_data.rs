//! Derive helper for generating namespaced instruction data types and a TryFrom<u8> for each
//! instruction enum variant.

use instruction_macros_impl::{
    parse::{
        instruction_variant::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        render_instruction_data,
        render_pack_into_slice_trait,
        render_try_from_u8,
    },
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub struct DeriveInstructionData {
    pub try_from_u8: TokenStream,
    pub pack_into_slice_trait: TokenStream,
    pub instruction_data: TokenStream,
}

pub fn derive_instruction_data(
    input: DeriveInput,
    as_instruction_events: bool,
) -> syn::Result<DeriveInstructionData> {
    let parsed_enum = ParsedEnum::new(input, as_instruction_events)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum)?;

    let try_from_u8 = render_try_from_u8(&parsed_enum, &instruction_variants);
    let instruction_data = render_instruction_data(&parsed_enum, instruction_variants);
    let pack_into_slice_trait = render_pack_into_slice_trait();

    Ok(DeriveInstructionData {
        try_from_u8,
        pack_into_slice_trait,
        instruction_data,
    })
}
