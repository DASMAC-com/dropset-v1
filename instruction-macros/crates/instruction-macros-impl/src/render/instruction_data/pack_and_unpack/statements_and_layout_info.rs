//! Builds intermediate representations describing layout, ordering, and serialization statements
//! used by pack/unpack code generation.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::parse::{
    instruction_variant::InstructionVariant,
    primitive_arg::PrimitiveArg,
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

                    let pack = pack_statement(arg_name, pack_offset, size);
                    let unpack = unpack_arg_assignment(arg_name, arg_type, unpack_offset, size);
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

/// Create the pack statement for each instruction argument.
fn pack_statement(name: &Ident, pack_offset: usize, size: usize) -> TokenStream {
    let size_lit = Literal::usize_unsuffixed(size);
    let start_lit = Literal::usize_unsuffixed(pack_offset);
    let end_lit = Literal::usize_unsuffixed(pack_offset + size);

    quote! {
        ::core::ptr::copy_nonoverlapping(
            (&self.#name.to_le_bytes()).as_ptr(),
            (&mut data[#start_lit..#end_lit]).as_mut_ptr() as *mut u8,
            #size_lit,
        );
    }
}

/// Create the layout doc string that indicates which bytes are being written to for a single arg.
fn layout_doc_comment(
    arg_name: &Ident,
    arg_type: &PrimitiveArg,
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

/// Build a field assignment for the `unpack` body; aka the `from_le_bytes` with the pointer offset.
fn unpack_arg_assignment(
    arg_name: &Ident,
    arg_type: &PrimitiveArg,
    unpack_offset: usize,
    size: usize,
) -> TokenStream {
    let size_lit = Literal::usize_unsuffixed(size);
    let offset_lit = Literal::usize_unsuffixed(unpack_offset);
    let parsed_type = arg_type.as_parsed_type();

    let ptr_with_offset = match unpack_offset {
        0 => quote! { p },
        _ => quote! { p.add(#offset_lit) },
    };

    quote! {
        let #arg_name = #parsed_type::from_le_bytes(*(#ptr_with_offset as *const [u8; #size_lit]));
    }
}
