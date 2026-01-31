//! Renders the code that deserializes raw instruction data into structured arguments for program
//! execution.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    parse::{
        error_path::ErrorPath,
        error_type::ErrorType,
    },
    render::pack_struct_fields::fully_qualified_unpack_trait,
};

/// Render the fallible `unpack_untagged*` method.
///
/// `unpack_untagged*` deserializes raw instruction data bytes into structured arguments according
/// to the corresponding instruction variant's instruction arguments.
pub fn render() -> TokenStream {
    let unpack_trait = fully_qualified_unpack_trait();

    let ErrorPath { base, variant: _ } = ErrorType::InvalidInstructionData.to_path();

    quote! {
        /// This method unpacks the instruction data that comes *after* the discriminant has
        /// already been peeled off of the front of the slice.
        /// Trailing bytes are ignored; the length must be sufficient, not exact.
        #[inline(always)]
        pub fn unpack_untagged(instruction_data: &[u8]) -> Result<Self, #base> {
            <Self as #unpack_trait>::unpack(instruction_data)
        }
    }
}
