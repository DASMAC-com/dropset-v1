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
        error_path::ErrorPath,
        error_type::ErrorType,
        primitive_arg::PrimitiveArg,
    },
    render::Feature,
};

impl ArgumentType {
    pub fn pack_statement(&self, arg_name: &Ident, offset: usize) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let offset_lit = Literal::usize_unsuffixed(offset);

        let src_bytes_slice_expression = match self {
            Self::PrimitiveArg(arg) => match arg {
                PrimitiveArg::Bool => {
                    quote! { if self.#arg_name { [1] } else { [0] } }
                }
                _ => quote! { self.#arg_name.to_le_bytes() },
            },
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

    pub fn unpack_statement(
        &self,
        arg_name: &Ident,
        offset: usize,
        feature: Feature,
    ) -> TokenStream {
        let size_lit = Literal::usize_unsuffixed(self.size());
        let offset_lit = Literal::usize_unsuffixed(offset);
        let parsed_type = self.as_parsed_type();

        let ptr_with_offset = match offset {
            0 => quote! { p },
            _ => quote! { p.add(#offset_lit) },
        };

        let ErrorPath { base, variant } = ErrorType::InvalidInstructionData.to_path(feature);

        match self {
            Self::PrimitiveArg(arg) => match arg {
                PrimitiveArg::Bool => quote! {
                    let #arg_name = match (*(#ptr_with_offset as *const u8)) {
                        0 => false,
                        1 => true,
                        _ => return Err(#base::#variant),
                    };
                },
                _ => quote! {
                    let #arg_name = #parsed_type::from_le_bytes(*(#ptr_with_offset as *const [u8; #size_lit]));
                },
            },
            Self::PubkeyBytes => quote! {
                let #arg_name = *(#ptr_with_offset as *const [u8; #size_lit]);
            },
        }
    }
}
