use syn::{
    spanned::Spanned,
    Attribute,
    DataEnum,
    Ident,
    Variant,
};

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_argument::InstructionArgument,
        instruction_discriminant::try_parse_instruction_discriminant,
        validation::{
            validate_accounts,
            validate_args,
        },
    },
    ACCOUNT_IDENTIFIER,
    ARGUMENT_IDENTIFIER,
};

#[derive(Clone, Debug)]
pub struct InstructionVariant {
    pub variant_name: Ident,
    pub arguments: Vec<InstructionArgument>,
    pub accounts: Vec<InstructionAccount>,
    pub no_accounts_or_args: bool,
    pub discriminant: u8,
}

impl TryFrom<(u8, &Variant)> for InstructionVariant {
    type Error = syn::Error;

    fn try_from(discriminant_and_variant: (u8, &Variant)) -> syn::Result<Self> {
        let (discriminant, variant) = discriminant_and_variant;
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
            discriminant,
        })
    }
}

pub fn parse_instruction_variants(data_enum: &DataEnum) -> syn::Result<Vec<InstructionVariant>> {
    // Implicit discriminants either start at 0 or the last variant that was explicitly set + 1.
    let mut implicit_discriminant = 0;

    data_enum
        .variants
        .iter()
        .map(|variant| {
            let discriminant = try_parse_instruction_discriminant(implicit_discriminant, variant)?;
            let instruction_variant = InstructionVariant::try_from((discriminant, variant))?;
            implicit_discriminant = discriminant + 1;

            Ok(instruction_variant)
        })
        .collect::<_>()
}
