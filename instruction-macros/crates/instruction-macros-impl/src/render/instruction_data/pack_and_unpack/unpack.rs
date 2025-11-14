//! Renders the code that deserializes raw instruction data into structured arguments for program
//! execution.

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::quote;
use syn::Ident;

use crate::{
    parse::{
        error_path::ErrorPath,
        error_type::ErrorType,
    },
    render::Feature,
};

/// Render the inner body of the fallible `unpack` method.
///
/// `unpack` deserializes raw instruction data bytes into structured arguments according to the
/// corresponding instruction variant's instruction arguments.
pub fn render(
    size_without_tag: &Literal,
    unpack_assignments: &[TokenStream],
    field_names: &[Ident],
    feature: Feature,
) -> TokenStream {
    let ErrorPath { base, variant } = ErrorType::InvalidInstructionData.to_path(feature);

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

    // `unpack` must be marked with the feature flag cfg because it's exposed at the top-level
    // module without one when it's parsed as an instruction event. For the default parsing
    // case, it's just a redundant flag.
    let feature_flag = quote! { #[cfg(feature = #feature)] };

    quote! {
        /// This method unpacks the instruction data that comes *after* the discriminant has
        /// already been peeled off of the front of the slice.
        /// Trailing bytes are ignored; the length must be sufficient, not exact.
        #feature_flag
        #[inline(always)]
        pub fn unpack(instruction_data: &[u8]) -> Result<Self, #base> {
            #unpack_body
        }
    }
}
