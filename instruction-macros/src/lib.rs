use quote::quote;
use syn::{
    parse_macro_input,
    DeriveInput,
};

mod derive;
mod parse;
mod render;

use parse::parsing_error::ParsingError;

use crate::{
    derive::{
        accounts::derive_accounts,
        instruction_data::derive_instruction_data,
    },
    render::*,
};

const ACCOUNT_IDENTIFIER: &str = "account";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";
const DESCRIPTION: &str = "desc";

#[proc_macro_derive(ProgramInstruction, attributes(account, args, sigil))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let (try_from_u8_render, instruction_data_render) = match derive_instruction_data(input.clone())
    {
        Ok(tag) => tag,
        Err(e) => return e.to_compile_error().into(),
    };

    let accounts_render = match derive_accounts(input) {
        Ok(accounts) => accounts,
        Err(e) => return e.to_compile_error().into(),
    };

    let merged_streams =
        merge_namespaced_token_streams(vec![instruction_data_render, accounts_render]);

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
        .collect::<proc_macro2::TokenStream>();

    quote! {
        #try_from_u8_render
        #namespaced_outputs
    }
    .into()
}
