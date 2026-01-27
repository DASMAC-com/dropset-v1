//! Renders the code that deserializes raw instruction data into structured arguments for program
//! execution.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::parse::{
    error_path::ErrorPath,
    error_type::ErrorType,
};

/// Render the fallible `unpack_*` method.
///
/// `unpack_*` deserializes raw instruction data bytes into structured arguments according to the
/// corresponding instruction variant's instruction arguments.
pub fn render(
    size_without_tag: &Literal,
    field_names: &[Ident],
    unpack_assignments: Vec<TokenStream>,
) -> TokenStream {
    let ErrorPath { base, variant } = ErrorType::InvalidInstructionData.to_path();

    let unpack_body = match size_without_tag.to_string().as_str() {
        // If the instruction has 0 bytes of data after the tag, simply return the Ok(empty data
        // struct) because all passed slices are valid.
        "0" => quote! { Ok(Self {}) },
        _ => quote! {
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
        },
    };

    quote! {
        /// This method unpacks the instruction data that comes *after* the discriminant has
        /// already been peeled off of the front of the slice.
        /// Trailing bytes are ignored; the length must be sufficient, not exact.
        #[inline(always)]
        pub fn unpack(instruction_data: &[u8]) -> Result<Self, #base> {
            #unpack_body
        }
    }
}
