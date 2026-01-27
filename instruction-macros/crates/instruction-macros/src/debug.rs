//! Internal debug utilities for inspecting generated macro output, including helpers to print
//! unique multi-segment paths from token streams. Not intended for public consumption.

use std::collections::HashSet;

use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    visit::Visit,
    File,
    Path,
};

#[derive(Default)]
struct Visitor {
    pub seen: HashSet<String>,
}

impl<'ast> Visit<'ast> for Visitor {
    fn visit_attribute(&mut self, _attr: &'ast syn::Attribute) {
        // Skip attribute contents entirely.
    }

    fn visit_path(&mut self, p: &'ast Path) {
        if p.segments.len() > 1 {
            let path_str = quote! { #p }.to_string().replace(" ", "");
            self.seen.insert(path_str);
        }
        syn::visit::visit_path(self, p);
    }
}

// Parse a token stream as a File. If it fails, wrap in a dummy module and try again.
fn parse_as_file(ts: &TokenStream) -> syn::Result<File> {
    let file = syn::parse2::<File>(ts.clone());
    let dummy_module = |_| syn::parse2::<File>(quote! { mod __x { #ts } });
    file.or_else(dummy_module)
}

/// Print all multi-segment paths in a slice of token streams, e.g. `foo::bar`
/// Deduplicates paths so as to only print unique paths.
///
/// Helpful for catching non-fully-qualified paths in the macro output.
///
/// Example output:
/// ```rust,ignore
/// ::core::mem::MaybeUninit::uninit
/// ::core::mem::MaybeUninit<u8>
/// ::core::ptr::copy_nonoverlapping
/// ::solana_program_error::ProgramResult
/// ::solana_account_view::AccountView
/// ::solana_instruction_view::cpi::invoke_signed
/// ::solana_instruction_view::InstructionView
/// ::solana_instruction_view::cpi::Signer
/// crate::program::ID
/// super::DropsetInstruction::CloseSeat
/// super::DropsetInstruction::Deposit
/// u16::from_le_bytes
/// u32::from_le_bytes
/// ```
#[doc(hidden)]
pub fn debug_print_multi_segment_paths(streams: &[&TokenStream]) {
    let mut v = Visitor::default();
    streams.iter().for_each(|tokens| {
        if let Ok(f) = parse_as_file(tokens) {
            v.visit_file(&f);
        } else {
            eprintln!("Couldn't parse file!");
        }
    });

    for path in v.seen.iter().sorted() {
        eprintln!("{path}");
    }
}
