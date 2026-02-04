//! Renders the core instruction data structures used by generated code, including argument packing,
//! unpacking, and documentation helpers.

mod pack_tagged_and_unpack;
mod struct_doc_comment;
mod unzipped_argument_infos;

use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use syn::Ident;

use crate::{
    parse::{
        argument_type::Size,
        instruction_argument::InstructionArgument,
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::{
        instruction_data::unzipped_argument_infos::InstructionArgumentInfo,
        pack_struct_fields::{
            fully_qualified_pack_trait,
            fully_qualified_tagged_trait,
            fully_qualified_unpack_trait,
        },
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
) -> TokenStream {
    instruction_variants
        .into_iter()
        // Don't render anything for instructions that have no accounts/arguments.
        .filter(|instruction_variant| instruction_variant.at_least_one_account_or_arg)
        .map(|instruction_variant| render_variant(parsed_enum, &instruction_variant))
        .collect::<_>()
}

fn render_variant(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
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

    let const_assertion =
        render_const_assertion(instruction_args, total_size_without_tag.clone(), &sizes);
    let size_with_tag_unsuffixed = Size::Lit(1).plus(total_size_without_tag);

    let (pack_tagged_fn, unpacks) =
        pack_tagged_and_unpack::render(parsed_enum, instruction_variant);

    let pack_trait = fully_qualified_pack_trait();
    let unpack_trait = fully_qualified_unpack_trait();
    let tagged_trait = fully_qualified_tagged_trait();

    // Outputs:
    // - The instruction data struct with doc comments
    // - The layout doc comment for `pack`
    // - The const assertion that the packed size equals the sum of its fields + 1 (the tag)
    // - The implementations for `pack` and `unpack`
    quote! {
        #struct_doc
        #[repr(C)]
        #[derive(#pack_trait, #unpack_trait, Clone, Debug, PartialEq, Eq)]
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

            #unpacks
        }

        impl #tagged_trait for #struct_name {
            type PackedTagged = [u8; #size_with_tag_unsuffixed];

            /// This is the instruction variant discriminant as a `u8` byte.
            const TAG_BYTE: u8 = #enum_ident::#tag_variant as u8;

            #pack_tagged_fn
        }

    }
}

fn render_const_assertion(
    instruction_args: &[InstructionArgument],
    total_size_without_tag: Size,
    sizes: &[Size],
) -> TokenStream {
    let total_with_tag = Size::Lit(1).plus(total_size_without_tag);

    if instruction_args.is_empty() {
        quote! { const _: [(); #total_with_tag] = [(); 1]; }
    } else {
        quote! { const _: [(); #total_with_tag] = [(); 1 + #( #sizes )+* ]; }
    }
}
