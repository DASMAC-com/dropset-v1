//! Derive helper for the [`crate::Pack`] trait.

use instruction_macros_impl::{
    parse::parsed_struct::ParsedStruct,
    render::render_pack_impl,
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_pack(input: DeriveInput) -> syn::Result<TokenStream> {
    let parsed_struct = ParsedStruct::new(input)?;

    Ok(render_pack_impl(parsed_struct))
}
