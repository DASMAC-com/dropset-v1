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

/// Renders the `invoke_`, `invoke_signed` for pinocchio/solana-program and the create
/// instruction method for the client.
pub fn render_invoke_methods(
    feature: Feature,
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
) -> TokenStream {
    let data_ident = instruction_variant.instruction_data_struct_ident();
    let accounts = &instruction_variant.accounts;
    let program_id_path = &parsed_enum.program_id_path;
    let (metas, names) = accounts
        .iter()
        .map(|acc| {
            (
                acc.render_account_meta(feature),
                format_ident!("{}", acc.name),
            )
        })
        .collect::<(Vec<_>, Vec<_>)>();

    match feature {
        Feature::Pinocchio => pinocchio_invoke(program_id_path, data_ident, metas, names),
        Feature::SolanaProgram => solana_program_invoke(program_id_path, data_ident, metas, names),
        Feature::Client => client_create_instruction(program_id_path, data_ident, metas),
    }
}

fn pinocchio_invoke(
    program_id_path: &Path,
    instruction_data_type: Ident,
    account_metas: Vec<TokenStream>,
    account_names: Vec<Ident>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_type) -> ::pinocchio::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[::pinocchio::instruction::Signer], data: #instruction_data_type) -> ::pinocchio::ProgramResult {
            let accounts = &[ #(#account_metas),* ];
            let Self {
                #(#account_names),*
            } = self;

            ::pinocchio::cpi::invoke_signed(
                &::pinocchio::instruction::Instruction {
                    program_id: &#program_id_path.into(),
                    accounts,
                    data: &data.pack(),
                },
                &[
                    #(#account_names),*
                ],
                signers_seeds,
            )
        }
    }
}

fn solana_program_invoke(
    program_id_path: &Path,
    instruction_data_ident: Ident,
    account_metas: Vec<TokenStream>,
    account_names: Vec<Ident>,
) -> TokenStream {
    let res = quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_ident) -> ::solana_sdk::entrypoint::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[&[&[u8]]], data: #instruction_data_ident) -> ::solana_sdk::entrypoint::ProgramResult {
            let accounts = [ #(#account_metas),* ].to_vec();
            let Self {
                #(#account_names),*
            } = self;

            ::solana_cpi::invoke_signed(
                &::solana_instruction::Instruction {
                    program_id: #program_id_path.into(),
                    accounts,
                    data: data.pack().to_vec(),
                },
                &[
                    #(#account_names.clone()),*
                ],
                signers_seeds,
            )
        }
    };

    res
}

fn client_create_instruction(
    program_id_path: &Path,
    instruction_data_ident: Ident,
    account_metas: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn create_instruction(&self, data: #instruction_data_ident) -> ::solana_instruction::Instruction {
            let accounts = [ #(#account_metas),* ].to_vec();

            ::solana_instruction::Instruction {
                program_id: #program_id_path.into(),
                accounts,
                data: data.pack().to_vec(),
            }
        }
    }
}
