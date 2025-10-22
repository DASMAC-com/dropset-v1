use syn::DeriveInput;

use crate::{
    parse::{
        instruction_variants::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::*,
};

pub fn derive_accounts(input: DeriveInput) -> syn::Result<Vec<NamespacedTokenStream>> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;
    let namespaced_streams = render_account_structs(&parsed_enum, instruction_variants);

    Ok(namespaced_streams)
}
