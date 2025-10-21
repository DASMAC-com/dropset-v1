use quote::quote;
use syn::DeriveInput;

use crate::{
    parse::{
        instruction_variants::parse_instruction_variants,
        parsed_enum::ParsedEnum,
    },
    render::{
        account_structs::render_account_struct_variants,
        feature_namespace::merge_namespaced_token_streams,
    },
};

pub fn derive_accounts(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let parsed_enum = ParsedEnum::try_from(input)?;
    let instruction_variants = parse_instruction_variants(&parsed_enum.data_enum)?;
    let namespaced_streams = render_account_struct_variants(&parsed_enum, instruction_variants);
    let merged_streams = merge_namespaced_token_streams(vec![namespaced_streams]);

    let namespaced_outputs = merged_streams
        .into_iter()
        .map(|(namespace, tokens)| {
            let feature = namespace.0;

            quote! {
                #[cfg(feature = #feature)]
                pub mod #namespace {
                    #(#tokens)*
                }
            }
        })
        .collect();

    Ok(namespaced_outputs)
}
