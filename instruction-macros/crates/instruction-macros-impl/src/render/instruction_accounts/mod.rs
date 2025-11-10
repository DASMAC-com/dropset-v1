//! Renders account-related helpers for each instruction, including account metadata, loaders, and
//! invocation methods.

mod account_loader;
mod account_meta;
mod invoke_methods;

use proc_macro2::{
    Literal,
    TokenStream,
};
use quote::{
    format_ident,
    quote,
};
use strum::IntoEnumIterator;

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_variant::InstructionVariant,
        parsed_enum::ParsedEnum,
    },
    render::{
        feature_namespace::{
            FeatureNamespace,
            NamespacedTokenStream,
        },
        instruction_accounts::{
            account_loader::render_account_loader,
            invoke_methods::render_invoke_methods,
        },
        Feature,
    },
};

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
    let (struct_level_field_comments, struct_fields) = instruction_variant
        .accounts
        .iter()
        .map(|account| {
            (
                render_struct_level_doc_comment_for_field(account),
                render_struct_field(account, &lifetimed_ref, &type_suffix),
            )
        })
        .collect::<(Vec<TokenStream>, Vec<TokenStream>)>();

    let first_doc_line = format!(
        " The invocation struct for a `{enum_ident}::{instruction_variant_name}` instruction."
    );

    let lifetime = feature.account_info_lifetime();

    let invoke_methods = render_invoke_methods(feature, parsed_enum, instruction_variant);
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
    account: &InstructionAccount,
    lifetimed_ref: &TokenStream,
    type_suffix: &TokenStream,
) -> TokenStream {
    let name = &format_ident!("{}", account.name);
    let trimmed_description = account.description.trim_start();
    // Ensure the comment is prepended with a space for the doc comment.
    let doc_comment = &format!(" {}", trimmed_description);

    if trimmed_description.is_empty() {
        quote! { pub #name: #lifetimed_ref #type_suffix, }
    } else {
        quote! {
            #[doc = #doc_comment]
            pub #name: #lifetimed_ref #type_suffix,
        }
    }
}
