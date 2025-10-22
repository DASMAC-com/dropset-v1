use itertools::Itertools;
use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use strum::IntoEnumIterator;
use syn::Ident;

use crate::{
    parse::{
        error_path::ErrorPath,
        error_type::ErrorType,
        instruction_account::InstructionAccount,
        instruction_variants::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::feature_namespace::{
        Feature,
        FeatureNamespace,
        NamespacedTokenStream,
    },
};

impl Feature {
    pub fn account_info_lifetime(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { 'a, 'info },
            Feature::Pinocchio => quote! { 'a },
            Feature::Client => quote! {},
        }
    }

    pub fn lifetimed_ref(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { &'a },
            Feature::Pinocchio => quote! { &'a },
            Feature::Client => quote! {},
        }
    }

    /// The specific account info type path, without the lifetimed ref prefixed to it.
    pub fn account_info_type_path(&self) -> TokenStream {
        match self {
            Feature::SolanaProgram => quote! { solana_sdk::account_info::AccountInfo<'info> },
            Feature::Pinocchio => quote! { pinocchio::account_info::AccountInfo },
            Feature::Client => quote! { solana_sdk::pubkey::Pubkey },
        }
    }

    /// Renders the `invoke_`, `invoke_signed` for pinocchio/solana-program and the create
    /// instruction method for the client.
    pub fn render_invoke_methods(&self, instruction_variant: &InstructionVariant) -> TokenStream {
        let data_ident = instruction_variant.instruction_data_struct_ident();
        let accounts = &instruction_variant.accounts;
        let metas = accounts
            .iter()
            .map(|acc| acc.render_account_meta(self))
            .collect_vec();

        let infos = accounts
            .iter()
            .map(|acc| {
                let account_ident = format_ident!("{}", acc.name);
                quote! { #account_ident }
            })
            .collect_vec();

        match self {
            Feature::Pinocchio => render_pinocchio_invoke(data_ident, metas, infos),
            Feature::SolanaProgram => render_solana_program_invoke(data_ident, metas, infos),
            Feature::Client => render_client_create_instruction(data_ident, metas),
        }
    }
}

pub fn render(
    parsed_enum: &ParsedEnum,
    instruction_variants: Vec<InstructionVariant>,
) -> Vec<NamespacedTokenStream> {
    instruction_variants
        .into_iter()
        // Don't render anything for instructions that have no accounts/arguments.
        .filter(|instruction_variant| !instruction_variant.no_accounts_or_args)
        .flat_map(|instruction_variant| {
            Feature::iter().map(move |feature| NamespacedTokenStream {
                tokens: render_variant(parsed_enum, &instruction_variant, feature),
                namespace: FeatureNamespace(feature),
            })
        })
        .collect()
}

fn render_variant(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
    feature: Feature,
) -> TokenStream {
    let enum_ident = &parsed_enum.enum_ident;
    let instruction_variant_name = &instruction_variant.variant_name;
    let struct_ident = format_ident!("{instruction_variant_name}");
    let type_suffix = feature.account_info_type_path();
    let lifetimed_ref = feature.lifetimed_ref();
    let struct_level_field_comments = instruction_variant
        .accounts
        .iter()
        .map(render_struct_level_doc_comment_for_field)
        .collect::<Vec<TokenStream>>();

    let struct_fields = instruction_variant
        .accounts
        .iter()
        .map(|account| {
            render_struct_field(
                &format_ident!("{}", account.name),
                // Ensure the comment is prepended with a space for the doc comment.
                &format!(" {}", &account.description.trim_start()),
                &lifetimed_ref,
                &type_suffix,
            )
        })
        .collect::<Vec<TokenStream>>();

    let first_doc_line = format!(
        " The invocation struct for a `{enum_ident}::{instruction_variant_name}` instruction."
    );

    let lifetime = feature.account_info_lifetime();
    // let (struct_and_lifetime, impl_and_lifetime) = feature.struct_and_impl_idents(&struct_ident);

    let invoke_methods = feature.render_invoke_methods(instruction_variant);
    let account_load_method = render_account_loader(feature, instruction_variant);

    quote! {
        #[doc = #first_doc_line]
        ///
        /// # Caller Guarantees
        ///
        /// When invoking this instruction as a cross-program invocation, caller must ensure that:
        /// - WRITE accounts are not currently borrowed in *any* capacity.
        /// - READ accounts are not currently mutably borrowed.
        ///
        /// ### Accounts
        #(#struct_level_field_comments)*
        pub struct #struct_ident<#lifetime> {
            #(#struct_fields)*
        }

        impl<#lifetime> #struct_ident<#lifetime> {
            #invoke_methods
            #account_load_method
        }
    }
}

/// Render an individual account's doc comment line that goes in the overall struct's doc comment
/// detailing each account's write/read and signer status.
///
/// Example output:
/// ```
/// /// 0. `[WRITE, SIGNER]` market_account
/// /// 1. `[WRITE]` base_mint_account
/// /// 2. `[READ]` quote_mint_account
/// ```
fn render_struct_level_doc_comment_for_field(account: &InstructionAccount) -> TokenStream {
    let index = Literal::u8_unsuffixed(account.index);
    let field_name = &account.name;
    let mutability_and_signer = match (account.is_writable, account.is_signer) {
        (true, true) => "`[WRITE, SIGNER]`",
        (true, false) => "`[WRITE]`",
        (false, true) => "`[READ, SIGNER]`",
        (false, false) => "`[READ]`",
    };
    let description = &account.description;
    let comment = format!(" {index}. {mutability_and_signer} `{field_name}` {description}");
    quote! { #[doc = #comment] }
}

fn render_struct_field(
    name: &Ident,
    doc_comment: &String,
    lifetimed_ref: &TokenStream,
    type_suffix: &TokenStream,
) -> TokenStream {
    if doc_comment.is_empty() {
        quote! { pub #name: #lifetimed_ref #type_suffix, }
    } else {
        quote! {
            #[doc = #doc_comment]
            pub #name: #lifetimed_ref #type_suffix,
        }
    }
}

impl InstructionAccount {
    fn render_account_meta(&self, account_struct: &Feature) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        match account_struct {
            Feature::Pinocchio => {
                let ctor_method = match (self.is_writable, self.is_signer) {
                    (true, true) => quote! { writable_signer },
                    (true, false) => quote! { writable },
                    (false, true) => quote! { readonly_signer },
                    (false, false) => quote! { readonly },
                };
                quote! { pinocchio::instruction::AccountMeta::#ctor_method(self.#field_ident.key()) }
            }
            Feature::SolanaProgram => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = format_ident!("{}", self.is_signer);
                quote! { solana_instruction::AccountMeta::#ctor_method(*self.#field_ident.key, #is_signer) }
            }
            Feature::Client => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = format_ident!("{}", self.is_signer);
                quote! { solana_instruction::AccountMeta::#ctor_method(self.#field_ident, #is_signer) }
            }
        }
    }
}

fn render_pinocchio_invoke(
    instruction_data_type: Ident,
    account_metas: Vec<TokenStream>,
    account_infos: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_type) -> pinocchio::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[pinocchio::instruction::Signer], data: #instruction_data_type) -> pinocchio::ProgramResult {
            let accounts = &[ #(#account_metas),* ];
            let Self {
                #(#account_infos),*
            } = self;

            pinocchio::cpi::invoke_signed(
                &pinocchio::instruction::Instruction {
                    program_id: &crate::program::ID,
                    accounts,
                    data: &data.pack(),
                },
                &[
                    #(#account_infos),*
                ],
                signers_seeds,
            )
        }
    }
}

fn render_solana_program_invoke(
    instruction_data_ident: Ident,
    account_metas: Vec<TokenStream>,
    account_infos: Vec<TokenStream>,
) -> TokenStream {
    let res = quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_ident) -> solana_sdk::entrypoint::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[&[&[u8]]], data: #instruction_data_ident) -> solana_sdk::entrypoint::ProgramResult {
            let accounts = [ #(#account_metas),* ].to_vec();
            let Self {
                #(#account_infos),*
            } = self;

            solana_cpi::invoke_signed(
                &solana_instruction::Instruction {
                    program_id: crate::program::ID.into(),
                    accounts,
                    data: data.pack().to_vec(),
                },
                &[
                    #(#account_infos.clone()),*
                ],
                signers_seeds,
            )
        }
    };

    res
}

fn render_client_create_instruction(
    instruction_data_ident: Ident,
    account_metas: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn create_instruction(&self, data: #instruction_data_ident) -> solana_instruction::Instruction {
            let accounts = [ #(#account_metas),* ].to_vec();

            solana_instruction::Instruction {
                program_id: crate::program::ID.into(),
                accounts,
                data: data.pack().to_vec(),
            }
        }
    }
}

/// Render the account load function.
///
/// The account load function fallibly attempts to structure a slice of `AccountInfo`s into the
/// corresponding struct of ordered accounts.
fn render_account_loader(
    feature: Feature,
    instruction_variant: &InstructionVariant,
) -> TokenStream {
    // The client doesn't need this function.
    if feature == Feature::Client {
        return quote! {};
    }

    let lifetimed_ref = feature.lifetimed_ref();
    let account_field_type = feature.account_info_type_path();
    let accounts = instruction_variant
        .accounts
        .iter()
        .map(|acc| format_ident!("{}", acc.name))
        .collect::<Vec<_>>();

    let ErrorPath { base, variant } = ErrorType::IncorrectNumAccounts.to_path(feature);

    assert!(!lifetimed_ref.is_empty(), "Method must receive a slice.");

    quote! {
        pub fn load_accounts(accounts: #lifetimed_ref [#account_field_type]) -> Result<Self, #base> {
            let [ #(#accounts),* ] = accounts else {
                return Err(#base::#variant);
            };

            Ok(Self {
                #(#accounts),*
            })
        }
    }
}
