//! Derive helper for generating the `try_from_tag` macro, `pack`, and `unpack` functions for
//! instruction event data.
//!
//! Notably, the structs for these instruction event data types do *not* implement invoke methods,
//! since they are solely for emitting event data inside a self-CPI instruction.
//!
//! Since `unpack` is only intended to be read client-side and `pack` is SDK-agnostic, in order to
//! simplify the generated code, the generated code is not namespaced and instead simply uses the
//! `solana_sdk` for `unpack`.

use instruction_macros_impl::{
    parse::{
        instruction_variant::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        merge_namespaced_token_streams,
        render_instruction_data,
        render_pack_into_slice_trait,
        render_try_from_tag_macro,
        Feature,
        FeatureNamespace,
    },
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub struct DeriveInstructionEventData {
    pub try_from_u8_macro: TokenStream,
    pub pack_into_slice_trait: TokenStream,
    pub client_instruction_data: TokenStream,
}

pub fn derive_instruction_event_data(
    input: DeriveInput,
) -> syn::Result<DeriveInstructionEventData> {
    let parsed_enum = ParsedEnum::try_from((true, input))?;
    let instruction_variants = parse_instruction_variants(&parsed_enum)?;

    let try_from_u8_macro = render_try_from_tag_macro(&parsed_enum, &instruction_variants);
    let instruction_data: Vec<instruction_macros_impl::render::NamespacedTokenStream> = render_instruction_data(&parsed_enum, instruction_variants);

    // Only use the client-side implementations to simplify and reduce the code generated. See the
    // module-level doc comment as to why.
    let merged_streams = merge_namespaced_token_streams(vec![instruction_data]);
    let client_streams = merged_streams
        .into_iter()
        .find(|d| d.0 == FeatureNamespace(Feature::Client))
        .unwrap()
        .1;

    let pack_into_slice_trait = render_pack_into_slice_trait();

    Ok(DeriveInstructionEventData {
        try_from_u8_macro,
        pack_into_slice_trait,
        client_instruction_data: quote::quote! { #(#client_streams)* },
    })
}
