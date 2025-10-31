mod pack;
mod statements_and_layout_info;
mod unpack;

use proc_macro2::TokenStream;
use statements_and_layout_info::*;
use syn::Ident;

use crate::{
    parse::{
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::Feature,
};

pub fn render(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
    field_names: &[Ident],
    feature: Feature,
) -> (TokenStream, TokenStream) {
    let enum_ident = &parsed_enum.enum_ident;
    let tag_variant = &instruction_variant.variant_name;
    let StatementsAndLayoutInfo {
        size_without_tag,
        size_with_tag,
        layout_docs,
        pack_statements,
        unpack_assignments,
    } = StatementsAndLayoutInfo::new(instruction_variant);

    let pack = pack::render(
        enum_ident,
        tag_variant,
        layout_docs,
        pack_statements,
        size_with_tag,
    );

    let unpack = unpack::render(&size_without_tag, &unpack_assignments, field_names, feature);

    (pack, unpack)
}
