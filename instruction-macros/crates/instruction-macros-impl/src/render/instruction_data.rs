use itertools::Itertools;
use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use strum::IntoEnumIterator;
use syn::{
    Ident,
    Type,
};

use crate::{
    parse::{
        error_path::ErrorPath,
        error_type::ErrorType,
        instruction_argument::InstructionArgument,
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::{
        feature_namespace::{
            FeatureNamespace,
            NamespacedTokenStream,
        },
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
    let error_base = ErrorType::InvalidInstructionData.to_path(feature).base;

    let struct_doc = build_struct_doc(enum_ident, tag_variant, instruction_args);

    let UnzippedArgumentInfos {
        names,
        types,
        sizes,
        doc_descriptions,
    } = UnzippedArgumentInfos::new(instruction_args);

    let (
        BuiltTokenStreams {
            layout_docs,
            pack_statements,
            unpack_assignments,
        },
        VariantSizes {
            size_with_tag,
            size_without_tag,
        },
    ) = build_token_streams_and_variant_sizes(instruction_args);

    let discriminant_description =
        format!(" - [0]: the discriminant `{enum_ident}::{tag_variant}` (u8, 1 byte)");
    let const_assertion = build_const_assertion(instruction_args, &size_with_tag, &sizes);

    let unpack_body = render_unpack_body(&size_without_tag, &unpack_assignments, &names, feature);

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

            #[doc = " Instruction data layout:"]
            #[doc = #discriminant_description]
            #(#layout_docs)*
            #[inline(always)]
            pub fn pack(&self) -> [u8; #size_with_tag] {
                let mut data: [core::mem::MaybeUninit<u8>; #size_with_tag] = [core::mem::MaybeUninit::uninit(); #size_with_tag];
                data[0].write(super::#enum_ident::#tag_variant as u8);
                // Safety: The pointers are non-overlapping and the same exact size.
                unsafe { #(#pack_statements)* }

                // All bytes initialized during the construction above.
                unsafe { *(data.as_ptr() as *const [u8; #size_with_tag]) }
            }

            /// This method unpacks the instruction data that comes *after* the discriminant has
            /// already been peeled off of the front of the slice.
            /// Trailing bytes are ignored; the length must be sufficient, not exact.
            #[inline(always)]
            pub fn unpack(instruction_data: &[u8]) -> Result<Self, #error_base> {
                #unpack_body
            }
        }
    }
}

fn render_unpack_body(
    size_without_tag: &Literal,
    unpack_assignments: &[TokenStream],
    field_names: &[Ident],
    feature: Feature,
) -> TokenStream {
    let error_path = ErrorType::InvalidInstructionData.to_path(feature);
    let ErrorPath { base, variant } = error_path;

    // If the instruction has 0 bytes of data after the tag, simply return the Ok(empty data struct)
    // because all passed slices are valid.
    if size_without_tag.to_string().as_str() == "0" {
        return quote! { Ok(Self {}) };
    }

    quote! {
        if instruction_data.len() < #size_without_tag {
            return Err(#base::#variant);
        }

        // Safety: The length was just verified; all dereferences are valid.
        unsafe {
            let p = instruction_data.as_ptr();
            #(#unpack_assignments)*

            Ok(Self {
                #(#field_names),*
            })
        }
    }
}

fn build_const_assertion(
    instruction_args: &[InstructionArgument],
    total_size_with_tag: &Literal,
    sizes: &[Literal],
) -> TokenStream {
    if instruction_args.is_empty() {
        quote! { const _: [(); #total_size_with_tag] = [(); 1]; }
    } else {
        quote! { const _: [(); #total_size_with_tag] = [(); 1 + #( #sizes )+* ]; }
    }
}

fn build_struct_doc(
    enum_ident: &Ident,
    tag_variant: &Ident,
    instruction_args: &[InstructionArgument],
) -> TokenStream {
    let first_line = format!(" `{}::{}` instruction data.", enum_ident, tag_variant);

    let remaining = instruction_args
        .iter()
        .map(|a| {
            let line = match a.description.is_empty() {
                true => format!(" - `{}`", a.name),
                false => format!(" - `{}` — {}", a.name, a.description),
            };
            quote! { #[doc = #line] }
        })
        .collect::<Vec<_>>();

    quote! {
        #[doc = #first_line]
        #(
            #[doc = ""]
            #remaining
        )*
    }
}

/// An unzipped collection of each instruction argument's identifying information.
///
/// For example, this struct might resemble something like this:
/// ```rust
/// UnzippedArgumentInfo {
///     names: ["amount", "index"],
///     parsed_types: [u64, u32],
///     sizes: [8, 4],
///     descriptions: ["The amount to deposit.", "The user's index."],
/// }
/// ```
struct UnzippedArgumentInfos {
    /// The field's name; e.g. `name` in `pub name: u32,`
    names: Vec<Ident>,
    /// The field's type; e.g. `u32`
    types: Vec<Type>,
    /// The literal token for the `usize` size; e.g. `4` for a `u32`
    sizes: Vec<Literal>,
    /// The doc comment description for this argument.
    doc_descriptions: Vec<TokenStream>,
}

impl UnzippedArgumentInfos {
    pub fn new(instruction_args: &[InstructionArgument]) -> Self {
        let (names, types, sizes, doc_descriptions) = instruction_args
            .iter()
            .map(|arg| {
                let doc_description = match arg.description.is_empty() {
                    true => quote! {},
                    false => {
                        let description = format!(" {}", arg.description);
                        quote! { #[doc = #description] }
                    }
                };
                let parsed_type = &arg.ty.as_parsed_type();
                let name = &arg.name;
                (
                    name.clone(),
                    parsed_type.clone(),
                    Literal::usize_unsuffixed(arg.ty.size()),
                    doc_description,
                )
            })
            .multiunzip();

        Self {
            names,
            types,
            sizes,
            doc_descriptions,
        }
    }
}

#[derive(Default)]
struct BuiltTokenStreams {
    /// The layout doc indicating which bytes the field occupies in the layout.
    pub layout_docs: Vec<TokenStream>,
    /// Each field's individual `pack` statement.
    pub pack_statements: Vec<TokenStream>,
    /// Each field's `unpack` assignment; e.g. `let field = ...`;
    pub unpack_assignments: Vec<TokenStream>,
}

struct VariantSizes {
    /// The total size including the tag.
    pub size_with_tag: Literal,
    /// The total size excluding the tag.
    pub size_without_tag: Literal,
}

fn build_token_streams_and_variant_sizes(
    instruction_args: &[InstructionArgument],
) -> (BuiltTokenStreams, VariantSizes) {
    let mut built_token_streams = BuiltTokenStreams::default();

    // The `0` is hardcoded for the discriminant, so start at byte `1..`
    let mut curr = 1;
    instruction_args.iter().for_each(|arg| {
        let name = &arg.name;
        let size = arg.ty.size();
        let start = curr;
        let end = curr + size;
        assert_eq!(end - start, size);

        let size_unsuff = Literal::usize_unsuffixed(size);
        let start_unsuff = Literal::usize_unsuffixed(start);
        let end_unsuff = Literal::usize_unsuffixed(end);

        let pack_statement = quote! {
            core::ptr::copy_nonoverlapping(
                (&self.#name.to_le_bytes()).as_ptr(),
                (&mut data[#start_unsuff..#end_unsuff]).as_mut_ptr() as *mut u8,
                #size_unsuff,
            );
        };

        // The pointer offset is for the instruction data which has already peeled the tag byte.
        let ptr_offset = start - 1;
        let ptr_with_offset = if ptr_offset == 0 {
            quote! { p }
        } else {
            let ptr_offset_unsuff = Literal::usize_unsuffixed(ptr_offset);
            quote! { p.add(#ptr_offset_unsuff) }
        };

        // Build the field assignment, aka the `from_le_bytes` with the pointer offset.
        let parsed_type = arg.ty.as_parsed_type();
        let unpack_assignment = quote! {
            let #name = #parsed_type::from_le_bytes(*(#ptr_with_offset as *const [u8; #size_unsuff]));
        };

        curr = end;

        // Build the layout doc statement that indicates which bytes are being written to.
        let pack_doc_string = if size == 1 {
            format!(
                " - [{}]: the `{}` ({}, 1 byte)",
                start, name, arg.ty
            )
        } else {
            format!(
                " - [{}..{}]: the `{}` ({}, {} bytes)",
                start, end, name, arg.ty, size
            )
    };

        let layout_doc = quote! {#[doc = #pack_doc_string]};

        built_token_streams.layout_docs.push(layout_doc);
        built_token_streams.pack_statements.push(pack_statement);
        built_token_streams
            .unpack_assignments
            .push(unpack_assignment);
    });

    let variant_sizes = VariantSizes {
        size_with_tag: Literal::usize_unsuffixed(curr),
        size_without_tag: Literal::usize_unsuffixed(curr - 1),
    };

    (built_token_streams, variant_sizes)
}
