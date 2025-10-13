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
use strum_macros::EnumIter;
use syn::Ident;

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_variants::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    FEATURE_CLIENT,
    FEATURE_PINOCCHIO_INVOKE,
    FEATURE_SOLANA_SDK_INVOKE,
};

#[derive(EnumIter)]
enum Sdk {
    SolanaSdkInvoke,
    PinocchioInvoke,
    Client,
}

impl Sdk {
    pub const fn feature(&self) -> &'static str {
        match self {
            Sdk::SolanaSdkInvoke => FEATURE_SOLANA_SDK_INVOKE,
            Sdk::PinocchioInvoke => FEATURE_PINOCCHIO_INVOKE,
            Sdk::Client => FEATURE_CLIENT,
        }
    }

    pub fn field_type_and_lifetime(&self) -> (TokenStream, Option<TokenStream>) {
        match self {
            Sdk::SolanaSdkInvoke => (
                quote! { solana_sdk::account_info::AccountInfo<'a> },
                Some(quote! { 'a }),
            ),
            Sdk::PinocchioInvoke => (
                quote! { &'a pinocchio::account_info::AccountInfo },
                Some(quote! { 'a }),
            ),
            Sdk::Client => (quote! { solana_sdk::pubkey::Pubkey }, None),
        }
    }

    pub fn render_impl(
        &self,
        data_struct_ident: &Ident,
        accounts: &[InstructionAccount],
    ) -> TokenStream {
        let acc_metas = accounts
            .iter()
            .map(|acc| acc.render_account_meta(self))
            .collect_vec();

        let acc_infos = accounts
            .iter()
            .map(|acc| {
                let account_ident = format_ident!("{}", acc.name);
                quote! { #account_ident, }
            })
            .collect_vec();

        match self {
            Sdk::PinocchioInvoke => render_pinocchio_impl(data_struct_ident, acc_metas, acc_infos),
            Sdk::SolanaSdkInvoke => render_solana_sdk_impl(data_struct_ident, acc_metas, acc_infos),
            Sdk::Client => render_client_impl(data_struct_ident, acc_metas),
        }
    }
}

pub fn render_account_struct_variants(
    parsed_enum: &ParsedEnum,
    instruction_variants: Vec<InstructionVariant>,
) -> TokenStream {
    instruction_variants
        .into_iter()
        // Don't render anything for `batch` instructions.
        .filter(|instruction_variant| !instruction_variant.no_accounts_or_args)
        .flat_map(|instruction_variant| {
            Sdk::iter().map(move |account_struct| {
                render_variant(parsed_enum, &instruction_variant, account_struct)
            })
        })
        .collect::<TokenStream>()
}

fn render_variant(
    parsed_enum: &ParsedEnum,
    instruction_variant: &InstructionVariant,
    sdk_type: Sdk,
) -> TokenStream {
    let enum_ident = &parsed_enum.enum_ident;
    let instruction_variant_name = &instruction_variant.variant_name;
    let struct_ident = format_ident!("{instruction_variant_name}");
    let feature = sdk_type.feature();
    let feature_cfg = quote! { #[cfg(feature = #feature)]};
    let (account_field_type, lifetime) = sdk_type.field_type_and_lifetime();
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
                &account_field_type,
            )
        })
        .collect::<Vec<TokenStream>>();

    let first_doc_line = format!(
        " The invocation struct for a `{enum_ident}::{instruction_variant_name}` instruction."
    );

    let (ident_and_explicit_lifetime, ident_and_elided_lifetime) =
        render_struct_name_and_lifetimes(&struct_ident, lifetime);

    let impl_render = sdk_type.render_impl(
        &instruction_variant.instruction_data_struct_ident(),
        &instruction_variant.accounts,
    );

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
        #feature_cfg
        pub struct #ident_and_explicit_lifetime {
            #(#struct_fields)*
        }

        #feature_cfg
        impl #ident_and_elided_lifetime {
            #impl_render
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
    account_info_type: &TokenStream,
) -> TokenStream {
    if doc_comment.is_empty() {
        quote! { pub #name: #account_info_type, }
    } else {
        quote! {
            #[doc = #doc_comment]
            pub #name: #account_info_type,
        }
    }
}

fn render_struct_name_and_lifetimes(
    struct_ident: &Ident,
    lifetime: Option<TokenStream>,
) -> (TokenStream, TokenStream) {
    let name_with_explicit_lifetime = match &lifetime {
        Some(lifetime) => quote! { #struct_ident<#lifetime> },
        None => quote! { #struct_ident },
    };

    let name_with_elided_lifetime = match &lifetime {
        Some(_) => quote! { #struct_ident<'_> },
        None => quote! { #struct_ident },
    };

    (name_with_explicit_lifetime, name_with_elided_lifetime)
}

impl InstructionAccount {
    fn render_account_meta(&self, account_struct: &Sdk) -> TokenStream {
        let field_ident = format_ident!("{}", self.name);
        match account_struct {
            Sdk::PinocchioInvoke => {
                let ctor_method = match (self.is_writable, self.is_signer) {
                    (true, true) => quote! { writable_signer },
                    (true, false) => quote! { writable },
                    (false, true) => quote! { readonly_signer },
                    (false, false) => quote! { readonly },
                };
                quote! { pinocchio::instruction::AccountMeta::#ctor_method(self.#field_ident.key()), }
            }
            Sdk::SolanaSdkInvoke => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = format_ident!("{}", self.is_signer);
                quote! { solana_instruction::AccountMeta::#ctor_method(*self.#field_ident.key, #is_signer), }
            }
            Sdk::Client => {
                let ctor_method = match self.is_writable {
                    true => quote! { new },
                    false => quote! { new_readonly },
                };
                let is_signer = format_ident!("{}", self.is_signer);
                quote! { solana_instruction::AccountMeta::#ctor_method(self.#field_ident, #is_signer), }
            }
        }
    }
}

fn render_pinocchio_impl(
    instruction_data_ident: &Ident,
    account_metas: Vec<TokenStream>,
    account_infos: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_ident) -> pinocchio::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[pinocchio::instruction::Signer], data: #instruction_data_ident) -> pinocchio::ProgramResult {
            let accounts = &[ #(#account_metas)* ];
            let Self {
                #(#account_infos)*
            } = self;

            pinocchio::cpi::invoke_signed(
                &pinocchio::instruction::Instruction {
                    program_id: &crate::program::ID,
                    accounts,
                    data: &data.pack(),
                },
                &[
                    #(#account_infos)*
                ],
                signers_seeds,
            )
        }
    }
}

fn render_solana_sdk_impl(
    instruction_data_ident: &Ident,
    account_metas: Vec<TokenStream>,
    account_infos: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn invoke(self, data: #instruction_data_ident) -> solana_sdk::entrypoint::ProgramResult {
            self.invoke_signed(&[], data)
        }

        #[inline(always)]
        pub fn invoke_signed(self, signers_seeds: &[&[&[u8]]], data: #instruction_data_ident) -> solana_sdk::entrypoint::ProgramResult {
            let accounts = [ #(#account_metas)* ].to_vec();
            let Self {
                #(#account_infos)*
            } = self;

            solana_cpi::invoke_signed(
                &solana_instruction::Instruction {
                    program_id: crate::program::ID.into(),
                    accounts,
                    data: data.pack().to_vec(),
                },
                &[
                    #(#account_infos)*
                ],
                signers_seeds,
            )
        }
    }
}

fn render_client_impl(
    instruction_data_ident: &Ident,
    account_metas: Vec<TokenStream>,
) -> TokenStream {
    quote! {
        #[inline(always)]
        pub fn create_instruction(&self, data: #instruction_data_ident) -> solana_instruction::Instruction {
            let accounts = [ #(#account_metas)* ].to_vec();

            solana_instruction::Instruction {
                program_id: crate::program::ID.into(),
                accounts,
                data: data.pack().to_vec(),
            }
        }
    }
}
