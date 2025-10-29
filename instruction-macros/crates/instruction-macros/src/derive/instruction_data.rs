use instruction_macros_impl::parse::parsed_enum::ParsedEnum;
use proc_macro2::TokenStream;
use syn::DeriveInput;

pub fn derive_instruction_data(input: DeriveInput) -> syn::Result<TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;

    Ok(TokenStream::new())
}
