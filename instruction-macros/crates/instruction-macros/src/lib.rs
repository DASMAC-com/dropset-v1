use quote::quote;
use syn::{
    parse_macro_input,
    DeriveInput,
};

mod derive;

use derive::{
    derive_accounts,
    derive_instruction_data,
};

#[proc_macro_derive(ProgramInstruction, attributes(account, args, program_id))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let instruction_data_render = match derive_instruction_data(input.clone()) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    let accounts_render = match derive_accounts(input) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    quote! {
        #accounts_render
        #instruction_data_render
    }
    .into()
}
