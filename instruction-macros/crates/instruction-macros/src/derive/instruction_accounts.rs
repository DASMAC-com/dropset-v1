//! Derive helper for generating namespaced instruction account structs and
//! account metas from an instruction enum definition.

use instruction_macros_impl::{
    parse::{
        instruction_variant::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        render_instruction_accounts,
        NamespacedTokenStream,
    },
};
use syn::DeriveInput;

pub fn derive_accounts(input: DeriveInput) -> syn::Result<Vec<NamespacedTokenStream>> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum)?;
    let accounts = render_instruction_accounts(&parsed_enum, instruction_variants);

    Ok(accounts)
}
