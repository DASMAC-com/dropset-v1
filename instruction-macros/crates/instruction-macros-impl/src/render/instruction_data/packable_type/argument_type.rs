use proc_macro2::Literal;
use quote::quote;

use crate::{
    parse::argument_type::{
        ArgumentType,
        ParsedPackableType,
    },
    render::instruction_data::packable_type::RenderedPackableType,
};

impl RenderedPackableType for ArgumentType {
    fn pack_statement(&self, arg_name: &syn::Ident, offset: usize) -> proc_macro2::TokenStream {
        match self {
            Self::PrimitiveArg(arg) => arg.pack_statement(arg_name, offset),
            Self::PubkeyBytes => {
                let size_lit = Literal::usize_unsuffixed(self.size());
                let start_lit = Literal::usize_unsuffixed(offset);
                let end_lit = Literal::usize_unsuffixed(offset + self.size());

                quote! {
                    ::core::ptr::copy_nonoverlapping(
                        (&self.#arg_name).as_ptr(),
                        (&mut data[#start_lit..#end_lit]).as_mut_ptr() as *mut u8,
                        #size_lit,
                    );
                }
            }
        }
    }

    fn unpack_statement(&self, arg_name: &syn::Ident, offset: usize) -> proc_macro2::TokenStream {
        match self {
            Self::PrimitiveArg(arg) => arg.unpack_statement(arg_name, offset),
            Self::PubkeyBytes => {
                let size_lit = Literal::usize_unsuffixed(self.size());
                let offset_lit = Literal::usize_unsuffixed(offset);

                let ptr_with_offset = match offset {
                    0 => quote! { p },
                    _ => quote! { p.add(#offset_lit) },
                };

                quote! {
                    let #arg_name = *(#ptr_with_offset as *const [u8; #size_lit]);
                }
            }
        }
    }
}
