use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::parse::argument_type::{
    ArgumentType,
    ParsedPackableType,
};

impl ArgumentType {
    pub fn pack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let offset_lit = Literal::usize_unsuffixed(offset);

        let src_bytes_slice_expression = match self {
            Self::PrimitiveArg(_) => quote! { self.#arg_name.to_le_bytes() },
            Self::PubkeyBytes => quote! { self.#arg_name },
        };

        quote! {
            ::core::ptr::copy_nonoverlapping(
                (#src_bytes_slice_expression).as_ptr(),
                (data.as_mut_ptr() as *mut u8).add(#offset_lit),
                #size_lit,
            );
        }
    }

    pub fn unpack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let offset_lit = Literal::usize_unsuffixed(offset);
        let parsed_type = self.as_parsed_type();

        let ptr_with_offset = match offset {
            0 => quote! { p },
            _ => quote! { p.add(#offset_lit) },
        };

        match self {
            Self::PrimitiveArg(_) => quote! {
                let #arg_name = #parsed_type::from_le_bytes(*(#ptr_with_offset as *const [u8; #size_lit]));
            },
            Self::PubkeyBytes => quote! {
                let #arg_name = *(#ptr_with_offset as *const [u8; #size_lit]);
            },
        }
    }
}
