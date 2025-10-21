use quote::quote;
use syn::{
    parse_macro_input,
    DeriveInput,
};

mod derive;
mod parse;
mod render;

use parse::parsing_error::ParsingError;

use crate::derive::{
    accounts::derive_accounts,
    instruction_data::derive_instruction_data,
};

const ACCOUNT_IDENTIFIER: &str = "account";
const CONFIG_ATTR: &str = "sigil";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";
const DESCRIPTION: &str = "desc";

#[proc_macro_derive(ProgramInstruction, attributes(account, args, sigil))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let tag_render = match derive_instruction_data(input.clone()) {
        Ok(tag) => tag,
        Err(e) => return e.to_compile_error().into(),
    };

    let accounts_render = match derive_accounts(input) {
        Ok(accounts) => accounts,
        Err(e) => return e.to_compile_error().into(),
    };

    quote! {
        #tag_render
        #accounts_render
    }
    .into()
}
