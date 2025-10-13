use quote::quote;
use syn::{parse_macro_input, DeriveInput};

mod derive;
mod parse;
mod render;

use parse::error::ParsingError;

use crate::derive::{accounts::derive_accounts, instruction_data::derive_instruction_data};

const ACCOUNT_IDENTIFIER: &str = "account";
const CONFIG_ATTR: &str = "program_instruction";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";
const DESCRIPTION: &str = "desc";
const DEFAULT_TAG_ERROR_BASE: &str = "ProgramError";
const DEFAULT_TAG_ERROR_TYPE: &str = "InvalidInstructionData";
const FEATURE_SOLANA_SDK_INVOKE: &str = "solana-sdk-invoke";
const FEATURE_PINOCCHIO_INVOKE: &str = "pinocchio-invoke";
const FEATURE_CLIENT: &str = "client";

#[proc_macro_derive(ProgramInstruction, attributes(account, args, program_instruction))]
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
