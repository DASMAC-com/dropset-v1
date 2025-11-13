//! Derive helper for generating namespaced instruction data types and a `try_from_u8`-style tag
//! macro from an instruction enum definition.

use instruction_macros_impl::{
    parse::{
        instruction_variant::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        render_instruction_data,
        render_try_from_tag_macro,
        NamespacedTokenStream,
    },
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub struct DeriveInstructionData {
    pub try_from_u8_macro: TokenStream,
    pub instruction_data: Vec<NamespacedTokenStream>,
}

pub fn derive_instruction_data(input: DeriveInput) -> syn::Result<DeriveInstructionData> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum)?;

    let try_from_u8_macro = render_try_from_tag_macro(&parsed_enum, &instruction_variants);
    let instruction_data = render_instruction_data(&parsed_enum, instruction_variants);

    Ok(DeriveInstructionData {
        try_from_u8_macro,
        instruction_data,
    })
}
