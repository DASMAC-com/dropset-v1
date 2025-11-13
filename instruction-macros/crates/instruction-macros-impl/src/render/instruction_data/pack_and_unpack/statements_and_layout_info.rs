//! Builds intermediate representations describing layout, ordering, and serialization statements
//! used by pack/unpack code generation.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::{
    parse::{
        argument_type::{
            ArgumentType,
            ParsedPackableType,
        },
        instruction_variant::InstructionVariant,
    },
    render::instruction_data::packable_type::RenderedPackableType,
};

pub struct StatementsAndLayoutInfo {
    /// The total size of the struct without the tag byte as a literal `usize`.
    pub size_without_tag: Literal,
    /// The total size of the struct with the tag byte as a literal `usize`.
    pub size_with_tag: Literal,
    /// The layout docs indicating which bytes each field occupies in the struct layout.
    pub layout_docs: Vec<TokenStream>,
    /// Each field's individual `pack` statement.
    pub pack_statements: Vec<TokenStream>,
    /// Each field's `unpack` assignment; e.g. `let field = ...`;
    pub unpack_assignments: Vec<TokenStream>,
}

impl StatementsAndLayoutInfo {
    pub fn new(instruction_variant: &InstructionVariant) -> StatementsAndLayoutInfo {
        let instruction_args = &instruction_variant.arguments;
        let (size_without_tag, layout_docs, pack_statements, unpack_assignments) =
            instruction_args.iter().fold(
                (0, vec![], vec![], vec![]),
                |(curr, mut layout_docs, mut pack_statements, mut unpack_assignments), arg| {
                    // Pack statements must also pack the discriminant first, so start at byte `1`
                    let pack_offset = curr + 1;
                    // Unpack statements operate on the instruction data *after* the tag byte has
                    // been peeled.
                    let unpack_offset = curr;

                    let arg_name = &arg.name;
                    let arg_type = &arg.ty;
                    let size = arg.ty.size();

                    let pack = arg_type.pack_statement(arg_name, pack_offset);
                    let unpack = arg_type.unpack_statement(arg_name, unpack_offset);
                    let layout_comment = layout_doc_comment(arg_name, arg_type, pack_offset, size);

                    layout_docs.push(layout_comment);
                    pack_statements.push(pack);
                    unpack_assignments.push(unpack);

                    (
                        curr + size,
                        layout_docs,
                        pack_statements,
                        unpack_assignments,
                    )
                },
            );

        StatementsAndLayoutInfo {
            size_without_tag: Literal::usize_unsuffixed(size_without_tag),
            size_with_tag: Literal::usize_unsuffixed(size_without_tag + 1),
            layout_docs,
            pack_statements,
            unpack_assignments,
        }
    }
}

/// Create the layout doc string that indicates which bytes are being written to for a single arg.
fn layout_doc_comment(
    arg_name: &Ident,
    arg_type: &ArgumentType,
    pack_offset: usize,
    size: usize,
) -> TokenStream {
    let end = pack_offset + size;
    let layout_doc_string = match size {
        1 => format!(
            " - [{}]: the `{}` ({}, 1 byte)",
            pack_offset, arg_name, arg_type
        ),
        size => format!(
            " - [{}..{}]: the `{}` ({}, {} bytes)",
            pack_offset, end, arg_name, arg_type, size
        ),
    };

    quote! { #[doc = #layout_doc_string] }
}
