use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::{
    parse::{
        instruction_tags::InstructionTags,
        instruction_variants::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::*,
};

pub fn derive_instruction_data(
    input: DeriveInput,
) -> syn::Result<(TokenStream, Vec<NamespacedTokenStream>)> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_tags = InstructionTags::try_from(&parsed_enum.data_enum)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;

    let instruction_data_variants =
        render_instruction_data_struct(&parsed_enum, instruction_variants);
    let tag_try_from = render_try_from_u8(&parsed_enum, &instruction_tags);

    Ok((tag_try_from, instruction_data_variants))
}
