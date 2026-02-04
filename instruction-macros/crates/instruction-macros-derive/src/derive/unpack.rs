//! Derive helper for the [`crate::Unpack`] trait.

use instruction_macros_impl::{
    parse::parsed_struct::ParsedStruct,
    render::render_unpack_impl,
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_unpack(input: DeriveInput) -> syn::Result<TokenStream> {
    let parsed_struct = ParsedStruct::new(input)?;

    Ok(render_unpack_impl(parsed_struct))
}
