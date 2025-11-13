//! Extracts and formats instruction argument metadata, like the argument's `name`, `type`, byte
//! `size`, and `description`. This metadata is then used in various code generation functions.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::{
    Ident,
    Type,
};

use crate::parse::{
    argument_type::ParsedPackableType,
    instruction_argument::InstructionArgument,
};

/// Information about each instruction argument, as well as the total size of the instruction data
/// (not including the tag byte).
///
/// For example, this struct might resemble something like this:
/// ```rust
/// InstructionArgumentInfo {
///     names: ["amount", "index"],
///     types: [u64, u32],
///     sizes: [8, 4],
///     descriptions: ["The amount to deposit.", "The user's index."],
///     total_size_without_tag: 13,
/// }
/// ```
#[derive(Default)]
pub struct InstructionArgumentInfo {
    /// Each argument's name; e.g. `name` in `pub name: u32,`
    pub names: Vec<Ident>,
    /// Each argument's type; e.g. `u32`
    pub types: Vec<Type>,
    /// Each argument's size as a literal.
    pub sizes: Vec<Literal>,
    /// Each argument's doc comment description.
    pub doc_descriptions: Vec<TokenStream>,
    /// The total size of all arguments, without the tag byte.
    pub total_size_without_tag: usize,
}

impl InstructionArgumentInfo {
    pub fn new(instruction_args: &[InstructionArgument]) -> Self {
        instruction_args
            .iter()
            .fold(InstructionArgumentInfo::default(), |mut info, arg| {
                let doc_description = match arg.description.is_empty() {
                    true => quote! {},
                    false => {
                        let description = format!(" {}", arg.description);
                        quote! { #[doc = #description] }
                    }
                };
                let parsed_type = &arg.ty.as_parsed_type();
                let name = &arg.name;
                let arg_size = arg.ty.size();

                info.names.push(name.clone());
                info.types.push(parsed_type.clone());
                info.sizes.push(Literal::usize_unsuffixed(arg_size));
                info.doc_descriptions.push(doc_description);
                info.total_size_without_tag += arg_size;

                info
            })
    }
}
