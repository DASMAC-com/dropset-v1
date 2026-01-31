//! Generates invoke-style helper methods for constructing and dispatching instructions using the
//! rendered account metadata. Each [`Feature`] type has its own specific output.

use proc_macro2::TokenStream;
use quote::{
    format_ident,
    quote,
};
use syn::{
    Ident,
    Path,
};

use crate::{
    parse::{
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::Feature,
};

/// Renders the `invoke_`, `invoke_signed` for program-based invocations and the create
/// instruction method for the client.
pub fn render_invoke_methods(
    feature: Feature,
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
) -> TokenStream {
    let data_ident = instruction_variant.instruction_data_struct_ident();
    let data_ident = quote! {
        super::#data_ident
    };
    let accounts = &instruction_variant.accounts;
    let program_id_path = &parsed_enum.program_id_path;
    let (accounts, names) = accounts
        .iter()
        .map(|acc| {
            (
                acc.render_instruction_account_view(feature),
                format_ident!("{}", acc.name),
            )
        })
        .collect::<(Vec<_>, Vec<_>)>();

    match feature {
        Feature::Program => invoke_functions(program_id_path, data_ident, accounts, names),
        Feature::Client => client_create_instruction(program_id_path, data_ident, accounts),
    }
}

fn invoke_functions(
    program_id_path: &Path,
    instruction_data_type: TokenStream,
    account_views: Vec<TokenStream>,
    account_names: Vec<Ident>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_type) -> ::solana_program_error::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[::solana_instruction_view::cpi::Signer], data: #instruction_data_type) -> ::solana_program_error::ProgramResult {
            let accounts = &[ #(#account_views),* ];
            let Self {
                #(#account_names),*
            } = self;

            ::solana_instruction_view::cpi::invoke_signed(
                &::solana_instruction_view::InstructionView {
                    program_id: &#program_id_path.into(),
                    accounts,
                    data: &data.pack_tagged(),
                },
                &[
                    #(#account_names),*
                ],
                signers_seeds,
            )
        }
    }
}

fn client_create_instruction(
    program_id_path: &Path,
    instruction_data_ident: TokenStream,
    account_views: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn create_instruction(&self, data: #instruction_data_ident) -> ::solana_instruction::Instruction {
            let accounts = [ #(#account_views),* ].to_vec();

            ::solana_instruction::Instruction {
                program_id: #program_id_path.into(),
                accounts,
                data: data.pack_tagged().to_vec(),
            }
        }
    }
}
