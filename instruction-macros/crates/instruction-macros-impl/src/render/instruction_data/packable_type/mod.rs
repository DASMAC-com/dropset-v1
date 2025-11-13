mod argument_type;
mod primitive_arg;

use proc_macro2::TokenStream;
use syn::Ident;

pub trait RenderedPackableType {
    /// Create the rendered output of the pack statement for an instruction argument.
    fn pack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream;

    /// Create the rendered output of the unpack field assignment for an instruction argument.
    fn unpack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream;
}
