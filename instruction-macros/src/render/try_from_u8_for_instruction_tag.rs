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

pub fn render(parsed_enum: &ParsedEnum, instruction_tags: &InstructionTags) -> TokenStream {
    let enum_ident = &parsed_enum.enum_ident;

    let sorted_by_discriminants = instruction_tags
        .0
        .clone()
        .into_iter()
        .sorted_by_key(|t| t.discriminant)
        .collect_vec();

    // Build a 2d collection of disjoint ranges, grouped by contiguous discriminants.
    // For example: [0..2, 3..5, 7..99]
    let chunks = sorted_by_discriminants
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
        impl #enum_ident {
            #[inline(always)]
            pub fn try_from_u8<E>(
                tag: u8,
                on_err: impl FnOnce() -> E,
            ) -> Result<Self, E> {
                // Safety: Match arms ensure only valid discriminants are transmuted.
                match tag {
                    #(#ranges)|* => Ok(unsafe { core::mem::transmute::<u8, Self>(tag) }),
                    _ => Err(on_err()),
                }
            }
        }
    }
}
