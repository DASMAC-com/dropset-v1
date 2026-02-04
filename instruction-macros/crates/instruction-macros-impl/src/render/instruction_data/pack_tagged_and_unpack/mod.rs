//! Code generation utilities for packing and unpacking instruction data, including field layout and
//! serialization logic.

mod pack_tagged;
mod statements_and_layout_info;
mod unpack;

use proc_macro2::TokenStream;
use statements_and_layout_info::*;

use crate::parse::{
    instruction_variant::InstructionVariant,
    parsed_enum::ParsedEnum,
};

/// Renders an enum instruction variant's `pack` function and each feature-based `unpack_*` method.
pub fn render(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
) -> (TokenStream, TokenStream) {
    let enum_ident = &parsed_enum.enum_ident;
    let tag_variant = &instruction_variant.variant_name;
    let StatementsAndLayoutInfo {
        size_with_tag,
        layout_docs,
    } = StatementsAndLayoutInfo::new(instruction_variant);

    let pack_tagged = pack_tagged::render(enum_ident, size_with_tag, tag_variant, layout_docs);

    let unpack = unpack::render();

    (pack_tagged, unpack)
}
