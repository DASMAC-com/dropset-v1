//! Proc-macro entrypoints for the various derives. They generate namespaced instruction data,
//! account layouts, and helper APIs from an instruction enum definition.
//!
//! [`ProgramInstruction`] also generates helper methods for cross-program invocations if the
//! `program` feature is enabled.
//!
//! [`ProgramInstructionEvent`] has the same output as [`ProgramInstruction`] but without the
//! helper methods and structs used for cross-program invocation.

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
    derive_pack,
    derive_unpack,
};

/// The entrypoint for the proc macro derive [`ProgramInstruction`].
#[proc_macro_derive(ProgramInstruction, attributes(account, args, program_id))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInstructionData {
        try_from_u8,
        pack_into_slice_trait,
        instruction_data,
    } = match derive_instruction_data(input.clone(), false) {
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

    debug_paths_if_env_var_set(&[&try_from_u8, &namespaced_outputs]);

    quote! {
        #try_from_u8
        #pack_into_slice_trait
        #instruction_data
        #namespaced_outputs
    }
    .into()
}

/// The entrypoint for the proc macro derive [`ProgramInstructionEvent`].
/// The primary difference between this and [`ProgramInstruction`] is that this derive won't
/// generate invocation functions.
#[proc_macro_derive(ProgramInstructionEvent, attributes(args, program_id))]
pub fn instruction_event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let DeriveInstructionData {
        try_from_u8,
        pack_into_slice_trait,
        instruction_data,
    } = match derive_instruction_data(input, true) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    debug_paths_if_env_var_set(&[&try_from_u8, &instruction_data]);

    quote! {
        #try_from_u8
        #pack_into_slice_trait
        #instruction_data
    }
    .into()
}

/// The entrypoint for the proc macro derive [`Pack`].
#[proc_macro_derive(Pack)]
pub fn pack(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let pack_impl = match derive_pack(input) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    debug_paths_if_env_var_set(&[&pack_impl]);

    pack_impl.into()
}

/// The entrypoint for the proc macro derive [`Unpack`].
#[proc_macro_derive(Unpack)]
pub fn unpack(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let unpack_impl = match derive_unpack(input) {
        Ok(render) => render,
        Err(e) => return e.into_compile_error().into(),
    };

    debug_paths_if_env_var_set(&[&unpack_impl]);

    unpack_impl.into()
}

/// Provides functionality for viewing all multi-segment paths.
/// A simple example of how to use this (note this silences the cargo expand output):
/// DEBUG_PATHS=1 cargo expand 1>/dev/null
///
/// See: [`debug::debug_print_multi_segment_paths`].
fn debug_paths_if_env_var_set(streams: &[&proc_macro2::TokenStream]) {
    if std::env::var("DEBUG_PATHS").is_ok() {
        debug::debug_print_multi_segment_paths(streams);
    }
}
