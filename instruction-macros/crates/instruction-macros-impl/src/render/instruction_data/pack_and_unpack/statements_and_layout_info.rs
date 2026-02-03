//! Builds intermediate representations describing layout, ordering, and serialization statements
//! used by pack/unpack code generation.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::parse::{
    argument_type::{
        ArgumentType,
        ParsedPackableType,
        Size,
    },
    instruction_variant::InstructionVariant,
};

pub struct StatementsAndLayoutInfo {
    /// The total [`Size`] of the struct with the tag byte.
    pub size_with_tag: Size,
    /// The layout docs indicating which bytes each field occupies in the struct layout.
    pub layout_docs: Vec<TokenStream>,
}

impl StatementsAndLayoutInfo {
    pub fn new(instruction_variant: &InstructionVariant) -> StatementsAndLayoutInfo {
        let instruction_args = &instruction_variant.arguments;
        let (size_without_tag, layout_docs) =
            instruction_args
                .iter()
                .fold((Size::Lit(0), vec![]), |(curr, mut layout_docs), arg| {
                    // Pack statements must also pack the discriminant first, so start at byte `1`
                    let pack_offset = curr.clone().plus(Size::Lit(1));

                    let arg_name = &arg.name;
                    let arg_type = &arg.ty;
                    let size = arg.ty.size();

                    let layout_comment =
                        layout_doc_comment(arg_name, arg_type, pack_offset, size.clone());

                    layout_docs.push(layout_comment);

                    (curr.plus(size), layout_docs)
                });

        StatementsAndLayoutInfo {
            size_with_tag: Size::Lit(1).plus(size_without_tag),
            layout_docs,
        }
    }
}

/// Create the layout doc string that indicates which bytes are being written to for a single arg.
fn layout_doc_comment(
    arg_name: &Ident,
    arg_type: &ArgumentType,
    pack_offset: Size,
    size: Size,
) -> TokenStream {
    let end = pack_offset.clone().plus(size.clone());
    let layout_doc_string = match size {
        Size::Lit(1) => format!(
            " - `[{}]` **{}** (`{}`, 1 byte)",
            pack_offset, arg_name, arg_type
        ),
        size => format!(
            " - `[{}..{}]` **{}** (`{}`, {} bytes)",
            pack_offset, end, arg_name, arg_type, size
        ),
    };

    quote! { #[doc = #layout_doc_string] }
}
