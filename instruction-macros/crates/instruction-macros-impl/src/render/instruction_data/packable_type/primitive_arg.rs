use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::{
    parse::{
        argument_type::ParsedPackableType,
        primitive_arg::PrimitiveArg,
    },
    render::instruction_data::packable_type::RenderedPackableType,
};

impl RenderedPackableType for PrimitiveArg {
    /// Render the pack statement for a primitive argument.
    fn pack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let start_lit = Literal::usize_unsuffixed(offset);
        let end_lit = Literal::usize_unsuffixed(offset + self.size());

        quote! {
            ::core::ptr::copy_nonoverlapping(
                (&self.#arg_name.to_le_bytes()).as_ptr(),
                (&mut data[#start_lit..#end_lit]).as_mut_ptr() as *mut u8,
                #size_lit,
            );
        }
    }

    /// Render the field assignment for a primitive argument's `unpack` body.
    fn unpack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let offset_lit = Literal::usize_unsuffixed(offset);
        let parsed_type = self.as_parsed_type();

        let ptr_with_offset = match offset {
            0 => quote! { p },
            _ => quote! { p.add(#offset_lit) },
        };

        quote! {
            let #arg_name = #parsed_type::from_le_bytes(*(#ptr_with_offset as *const [u8; #size_lit]));
        }
    }
}
