use instruction_macros_impl::{
    parse::{
        instruction_variant::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::render_try_from_tag_macro,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn derive_instruction_data(input: DeriveInput) -> syn::Result<TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;

    let try_from_u8_macro = render_try_from_tag_macro(&parsed_enum, &instruction_variants);

    Ok(quote! {
        #try_from_u8_macro
    })
}
