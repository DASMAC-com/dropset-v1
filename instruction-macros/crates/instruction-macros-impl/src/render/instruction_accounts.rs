use proc_macro2::TokenStream;
use quote::quote;
use strum::IntoEnumIterator;

use crate::{
    parse::{
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::feature_namespace::{
        Feature,
        FeatureNamespace,
        NamespacedTokenStream,
    },
};
pub fn render(
    parsed_enum: &ParsedEnum,
    instruction_variants: Vec<InstructionVariant>,
) -> Vec<NamespacedTokenStream> {
    instruction_variants
        .into_iter()
        // Don't render anything for instructions that have no accounts/arguments.
        .filter(|instruction_variant| !instruction_variant.no_accounts_or_args)
        .flat_map(|instruction_variant| {
            Feature::iter().map(move |feature| NamespacedTokenStream {
                tokens: render_variant(parsed_enum, &instruction_variant, feature),
                namespace: FeatureNamespace(feature),
            })
        })
        .collect()
}

fn render_variant(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
    feature: Feature,
) -> TokenStream {
    quote! {}
}
