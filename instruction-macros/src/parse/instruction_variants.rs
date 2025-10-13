use syn::{spanned::Spanned, Attribute, DataEnum, Ident, Variant};

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_argument::InstructionArgument,
        validation::{validate_accounts, validate_args},
    },
    ACCOUNT_IDENTIFIER, ARGUMENT_IDENTIFIER,
};

#[derive(Clone, Debug)]
pub struct InstructionVariant {
    pub variant_name: Ident,
    pub arguments: Vec<InstructionArgument>,
    pub accounts: Vec<InstructionAccount>,
    pub no_accounts_or_args: bool,
}

impl TryFrom<&Variant> for InstructionVariant {
    type Error = syn::Error;

    fn try_from(variant: &Variant) -> syn::Result<Self> {
        let arguments = variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident(ARGUMENT_IDENTIFIER))
            .map(Attribute::parse_args)
            .collect::<syn::Result<Vec<InstructionArgument>>>()?;

        validate_args(&arguments, variant.span())?;

        let accounts = variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident(ACCOUNT_IDENTIFIER))
            .enumerate()
            .map(InstructionAccount::try_from)
            .collect::<syn::Result<Vec<InstructionAccount>>>()?;

        // Don't validate empty instructions, because they won't generate any code.
        let no_accounts_or_args = arguments.is_empty() && accounts.is_empty();
        if !no_accounts_or_args {
            validate_accounts(&accounts, variant.span())?;
        }

        Ok(Self {
            variant_name: variant.ident.clone(),
            arguments,
            accounts,
            no_accounts_or_args,
        })
    }
}

pub fn parse_instruction_variants(data_enum: &DataEnum) -> syn::Result<Vec<InstructionVariant>> {
    data_enum
        .variants
        .iter()
        .map(InstructionVariant::try_from)
        .collect::<_>()
}
