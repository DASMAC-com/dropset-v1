use syn::DeriveInput;

use crate::{
    parse::{instruction_variants::parse_instruction_variants, parsed_enum::ParsedEnum},
    render::account_structs::render_account_struct_variants,
};

pub fn derive_accounts(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;

    Ok(render_account_struct_variants(
        &parsed_enum,
        instruction_variants,
    ))
}
