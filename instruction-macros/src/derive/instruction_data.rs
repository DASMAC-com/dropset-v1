use quote::quote;
use syn::DeriveInput;

use crate::{
    parse::{
        instruction_tags::InstructionTags,
        instruction_variants::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        instruction_data_struct::render_instruction_data_structs,
        try_from_u8_for_instruction_tag::render_try_from_u8_for_instruction_tag,
    },
};

pub fn derive_instruction_data(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_tags = InstructionTags::try_from(&parsed_enum.data_enum)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;
    let instruction_data_variants =
        render_instruction_data_structs(&parsed_enum, instruction_variants);
    let tag_try_from = render_try_from_u8_for_instruction_tag(&parsed_enum, instruction_tags);

    Ok(quote! {
        #tag_try_from
        #instruction_data_variants
    })
}
