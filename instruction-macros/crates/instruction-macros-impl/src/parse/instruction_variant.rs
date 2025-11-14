//! Parses and validates instruction enum variants into a structured, in-memory model.
//!
//! Each variant is analyzed for its discriminant, arguments, and accounts, then converted
//! into a validated representation as an [`InstructionVariant`]— the core input for instruction
//! code generation.

use syn::{
    spanned::Spanned,
    Attribute,
    Ident,
    Variant,
};

use crate::{
    parse::{
        instruction_account::InstructionAccount,
        instruction_argument::InstructionArgument,
        instruction_discriminant::try_parse_instruction_discriminant,
        parsed_enum::ParsedEnum,
        parsing_error::ParsingError,
        validation::{
            validate_accounts,
            validate_args,
        },
    },
    ACCOUNT_IDENTIFIER,
    ARGUMENT_IDENTIFIER,
};

/// A parsed and validated struct representing each instruction variant's validated name,
/// discriminant, arguments, and accounts.
///
/// This is the core model for each instruction variant's generated code.
#[derive(Clone, Debug)]
pub struct InstructionVariant {
    pub variant_name: Ident,
    pub arguments: Vec<InstructionArgument>,
    pub accounts: Vec<InstructionAccount>,
    pub at_least_one_account_or_arg: bool,
    pub discriminant: u8,
}

struct VariantInfo<'a> {
    pub discriminant: u8,
    pub variant: &'a Variant,
    pub as_instruction_events: bool,
}

impl TryFrom<VariantInfo<'_>> for InstructionVariant {
    type Error = syn::Error;

    fn try_from(variant_info: VariantInfo) -> syn::Result<Self> {
        let VariantInfo {
            discriminant,
            variant,
            as_instruction_events,
        } = variant_info;
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

        // Only validate instructions with at least one account or argument, since instructions
        // with neither won't generate any code.
        let at_least_one_account_or_arg = !arguments.is_empty() || !accounts.is_empty();
        if at_least_one_account_or_arg {
            validate_accounts(as_instruction_events, &accounts, variant.span())?;
        }

        // If this variant is parsed as an instruction event, it must have zero accounts.
        if as_instruction_events && !accounts.is_empty() {
            return Err(ParsingError::InstructionEventHasAccounts.new_err(variant.span()));
        }

        Ok(Self {
            variant_name: variant.ident.clone(),
            arguments,
            accounts,
            at_least_one_account_or_arg,
            discriminant,
        })
    }
}

/// Parses all variants of an instruction enum, assigning discriminants (explicit or implicit)
/// and converting them into validated instruction representations.
///
/// The variants can be parsed as one of the following:
/// - instruction data:       the arguments passed to an instruction, plus the instruction accounts
/// - instruction event data: the arguments passed to an instruction, no instruction accounts
pub fn parse_instruction_variants(
    parsed_enum: &ParsedEnum,
) -> syn::Result<Vec<InstructionVariant>> {
    let (data_enum, as_instruction_events) =
        (&parsed_enum.data_enum, parsed_enum.as_instruction_events);
    // Implicit discriminants either start at 0 or the last variant that was explicitly set + 1.
    let mut implicit_discriminant = 0;

    data_enum
        .variants
        .iter()
        .map(|variant| {
            let discriminant = try_parse_instruction_discriminant(implicit_discriminant, variant)?;
            let variant_info = VariantInfo {
                discriminant,
                variant,
                as_instruction_events,
            };
            let instruction_variant = InstructionVariant::try_from(variant_info)?;
            implicit_discriminant = discriminant + 1;

            Ok(instruction_variant)
        })
        .collect::<_>()
}
