use instruction_macros_impl::render::merge_namespaced_token_streams;
use quote::quote;
use syn::{
    parse_macro_input,
    DeriveInput,
};

mod debug;
mod derive;

use derive::{
    derive_accounts,
    derive_instruction_data,
};

use crate::derive::DeriveInstructionData;

#[proc_macro_derive(ProgramInstruction, attributes(account, args, program_id))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInstructionData {
        try_from_u8_macro,
        instruction_data,
    } = match derive_instruction_data(input.clone()) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    let accounts_render = match derive_accounts(input) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    let merged_streams = merge_namespaced_token_streams(vec![instruction_data, accounts_render]);

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

    debug::debug_print_multi_segment_paths(&try_from_u8_macro);
    debug::debug_print_multi_segment_paths(&namespaced_outputs);

    quote! {
        #try_from_u8_macro
        #namespaced_outputs
    }
    .into()
}
