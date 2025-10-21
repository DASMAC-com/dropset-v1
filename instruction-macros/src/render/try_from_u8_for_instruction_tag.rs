use itertools::Itertools;
use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;

use crate::parse::{
    instruction_tags::InstructionTags,
    parsed_enum::ParsedEnum,
};

pub fn render_try_from_u8_for_instruction_tag(
    parsed_enum: &ParsedEnum,
    instruction_tags: InstructionTags,
) -> TokenStream {
    let enum_ident = &parsed_enum.enum_ident;
    let error_base = &parsed_enum.config.errors.invalid_tag.base;
    let error_variant = &parsed_enum.config.errors.invalid_tag.variant;

    let mut cloned_variants = instruction_tags.0.clone().into_iter().collect_vec();
    cloned_variants.sort_by_key(|t| t.discriminant);

    // Build a 2d vector of disjoint ranges, grouped/chunked by contiguous discriminants.
    // For example: [0..2, 3..5, 7..99]
    let chunks = cloned_variants
        .chunk_by(|a, b| a.discriminant + 1 == b.discriminant)
        .collect_vec();

    let ranges = chunks.iter().map(|chunk| {
        let start = Literal::u8_unsuffixed(chunk[0].discriminant);
        if chunk.len() == 1 {
            quote! { #start }
        } else {
            let end =
                Literal::u8_unsuffixed(chunk.last().expect("Should have 1+ elements").discriminant);
            quote! { #start..=#end }
        }
    });

    quote! {
        impl TryFrom<u8> for #enum_ident {
            type Error = #error_base;

            fn try_from(tag: u8) -> Result<Self, Self::Error> {
                // Safety: Match arms ensure only valid discriminants are transmuted.
                match tag {
                    #(#ranges)|* => Ok(unsafe { core::mem::transmute::<u8, Self>(tag) }),
                    _ => Err(#error_base::#error_variant),
                }
            }
        }
    }
}
