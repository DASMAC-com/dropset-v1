use instruction_macros_impl::parse::{
    instruction_variant::parse_instruction_variants,
    parsed_enum::ParsedEnum,
};
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_instruction_data(input: DeriveInput) -> syn::Result<TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;

    Ok(TokenStream::new())
}
