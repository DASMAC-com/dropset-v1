mod pack_and_unpack;
mod struct_doc_comment;
mod unzipped_argument_infos;

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use strum::IntoEnumIterator;
use syn::Ident;

use crate::{
    parse::{
        instruction_argument::InstructionArgument,
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::{
        feature_namespace::{
            FeatureNamespace,
            NamespacedTokenStream,
        },
        instruction_data::unzipped_argument_infos::InstructionArgumentInfo,
        Feature,
    },
};

impl InstructionVariant {
    pub fn instruction_data_struct_ident(&self) -> Ident {
        format_ident!("{}InstructionData", &self.variant_name)
    }
}

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
        .collect::<_>()
}

fn render_variant(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
    feature: Feature,
) -> TokenStream {
    let tag_variant = &instruction_variant.variant_name;
    let struct_name = instruction_variant.instruction_data_struct_ident();
    let instruction_args = &instruction_variant.arguments;

    let enum_ident = &parsed_enum.enum_ident;

    let struct_doc = struct_doc_comment::render(enum_ident, tag_variant, instruction_args);

    let InstructionArgumentInfo {
        names,
        types,
        sizes,
        doc_descriptions,
        total_size_without_tag,
    } = InstructionArgumentInfo::new(instruction_args);

    let const_assertion = render_const_assertion(instruction_args, total_size_without_tag, &sizes);

    let (pack_fn, unpack_fn) =
        pack_and_unpack::render(parsed_enum, instruction_variant, &names, feature);

    // Outputs:
    // - The instruction data struct with doc comments
    // - The layout doc comment for `pack`
    // - The const assertion that the packed size equals the sum of its fields + 1 (the tag)
    // - The implementations for `pack` and `unpack`
    quote! {
        #struct_doc
        pub struct #struct_name {
            #(
                #doc_descriptions
                pub #names: #types,
            )*
        }

        /// Compile time assertion that the size with the tag == the sum of the field sizes.
        #const_assertion

        impl #struct_name {
            #struct_doc
            #[inline(always)]
            pub fn new(
                #(#names: #types),*
            ) -> Self {
                Self { #(#names),* }
            }

            #pack_fn

            #unpack_fn
        }
    }
}

fn render_const_assertion(
    instruction_args: &[InstructionArgument],
    total_size_without_tag: usize,
    sizes: &[Literal],
) -> TokenStream {
    let total_with_tag = Literal::usize_unsuffixed(total_size_without_tag + 1);

    if instruction_args.is_empty() {
        quote! { const _: [(); #total_with_tag] = [(); 1]; }
    } else {
        quote! { const _: [(); #total_with_tag] = [(); 1 + #( #sizes )+* ]; }
    }
}
