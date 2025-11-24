//! Proc-macro entrypoint for `#[derive(ProgramInstruction)]`, generating
//! namespaced instruction data, account layouts, and helper APIs from an
//! instruction enum definition.

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
    DeriveInstructionData,
};

use crate::derive::{
    derive_instruction_event_data,
    DeriveInstructionEventData,
};

#[proc_macro_derive(ProgramInstruction, attributes(account, args, program_id))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInstructionData {
        try_from_u8_macro,
        pack_into_slice_trait,
        instruction_data,
    } = match derive_instruction_data(input.clone()) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    let accounts_render = match derive_accounts(input) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    let merged_streams = merge_namespaced_token_streams(vec![accounts_render]);

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

    // Simple command to view all multi-segment paths (note this silences the cargo expand output):
    // DEBUG_PATHS=1 cargo expand 1>/dev/null
    if std::env::var("DEBUG_PATHS").is_ok() {
        debug::debug_print_multi_segment_paths(&[&try_from_u8_macro, &namespaced_outputs]);
    }

    quote! {
        #try_from_u8_macro
        #pack_into_slice_trait
        #instruction_data
        #namespaced_outputs
    }
    .into()
}

#[proc_macro_derive(ProgramInstructionEvent, attributes(args, program_id))]
pub fn instruction_event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInstructionEventData {
        try_from_u8_macro,
        pack_into_slice_trait,
        instruction_data,
    } = match derive_instruction_event_data(input.clone()) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    // Simple command to view all multi-segment paths (note this silences the cargo expand output):
    // DEBUG_PATHS=1 cargo expand 1>/dev/null
    if std::env::var("DEBUG_PATHS").is_ok() {
        debug::debug_print_multi_segment_paths(&[&try_from_u8_macro, &instruction_data]);
    }

    quote! {
        #try_from_u8_macro
        #pack_into_slice_trait
        #instruction_data
    }
    .into()
}
