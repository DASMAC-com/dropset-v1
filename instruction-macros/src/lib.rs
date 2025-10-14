use std::str::FromStr;

use itertools::Itertools;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Attribute, DataEnum, DeriveInput, Error, Expr, ExprLit, Ident, Lit, LitInt, Meta, PatLit,
    Result, Token, Type, Variant,
};

use crate::output::create_instruction_tags;

mod output;

const ACCOUNT_IDENTIFIER: &str = "account";
const INSTRUCTION_TAG: &str = "instruction_tag";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";
const DESCRIPTION: &str = "desc";

#[proc_macro_derive(ProgramInstructions, attributes(account, args, instruction_tag))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_item = match input.data {
        syn::Data::Enum(e) => e,
        _ => {
            return ParsingError::NotAnEnum
                .into_err(input.span())
                .to_compile_error()
                .into();
        }
    };

    let tag_output = if let Some(tag_config) = parse_instruction_tag_attr(&input.attrs) {
        impl_instruction_tag(input.ident, enum_item.clone(), tag_config)
            .unwrap_or_else(|e| e.to_compile_error())
    } else {
        quote! {}
    };

    let acc_and_args_output =
        impl_accounts_and_args(enum_item).unwrap_or_else(|e| e.to_compile_error());

    acc_and_args_output.into()
}

struct InstructionTagConfig {
    name: Ident,
    error: Option<Expr>,
}

fn parse_instruction_tag_attr(attrs: &[Attribute]) -> Option<InstructionTagConfig> {
    attrs
        .iter()
        .find(|attr| attr.path().is_ident(INSTRUCTION_TAG))
        .map(|attr| args.parse_args::<InstructionTagConfig>())
}

fn impl_instruction_tag(
    enum_ident: Ident,
    enum_item: DataEnum,
    tag_config: InstructionTagConfig,
) -> syn::Result<proc_macro2::TokenStream> {
    let instruction_tags = InstructionTags::try_from(enum_item.variants)?;
    let tag_tokens = create_instruction_tags(enum_ident, instruction_tags);
    eprintln!("{tag_tokens}");

    Ok(tag_tokens)
}

fn impl_accounts_and_args(enum_item: DataEnum) -> syn::Result<proc_macro2::TokenStream> {
    enum_item.variants.iter().try_for_each(|variant| {
        // Filter by attrs matching `#[account(...)]`, then try converting to `InstructionAccount`s.
        let instruction_accounts = variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident(ACCOUNT_IDENTIFIER))
            .enumerate()
            .map(InstructionAccount::try_from)
            .collect::<Result<Vec<InstructionAccount>>>()?;

        // Filter by attrs matching `#[args(...)]`, then try converting to `InstructionArgument`s.
        let instruction_arguments = variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident(ARGUMENT_IDENTIFIER))
            .map(Attribute::parse_args)
            .collect::<Result<Vec<InstructionArgument>>>()?;

        instruction_arguments.iter().for_each(|arg| {
            eprintln!("{:?}", arg);
        });

        validate_accounts(instruction_accounts, variant.span())?;
        validate_args(instruction_arguments, variant.span())?;

        Ok::<(), Error>(())
    })?;

    Ok(quote! {})
}

#[derive(Clone)]
struct InstructionTags(pub Vec<InstructionVariant>);

#[derive(Clone)]
struct InstructionVariant {
    name: String,
    discriminant: u8,
}

impl TryFrom<Punctuated<Variant, Comma>> for InstructionTags {
    type Error = Error;

    fn try_from(variants: Punctuated<Variant, Comma>) -> std::result::Result<Self, Self::Error> {
        // Implicit discriminants either start at 0 or the last variant that was explicitly set + 1.
        let mut implicit_discriminant = 0;

        let with_discriminants = variants
            .iter()
            .map(|variant| {
                if !variant.fields.is_empty() {
                    return Err(ParsingError::EnumVariantShouldBeFieldless.into_err(variant.span()));
                }

                let discriminant = match &variant.discriminant {
                    Some((_eq, expr)) => match expr {
                        Expr::Lit(ExprLit { lit, .. }) => match lit {
                            Lit::Int(lit) => lit
                                .base10_parse()
                                .or(Err(ParsingError::InvalidLiteralU8.into_err(lit.span())))?,
                            lit => return Err(ParsingError::InvalidLiteralU8.into_err(lit.span())),
                        },
                        expr => return Err(ParsingError::InvalidLiteralU8.into_err(expr.span())),
                    },
                    None => implicit_discriminant,
                };

                implicit_discriminant += 1;

                Ok(InstructionVariant {
                    name: variant.ident.to_string(),
                    discriminant,
                })
            })
            .collect::<Result<_>>()?;

        Ok(InstructionTags(with_discriminants))
    }
}

#[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
enum PrimitiveArg {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl TryFrom<&Type> for PrimitiveArg {
    type Error = Error;

    fn try_from(ty: &Type) -> std::result::Result<Self, Self::Error> {
        let err = ParsingError::InvalidPrimitiveType.into_err(ty.span());
        if let Type::Path(type_path) = ty {
            // No qualified paths, only primitives.
            if type_path.qself.is_some() {
                return Err(err);
            }
            // Only one segment, no `::` anywhere.
            if type_path.path.segments.len() != 1 {
                return Err(err);
            }
            // No generics allowed.
            let segment = &type_path.path.segments[0];
            if !segment.arguments.is_empty() {
                return Err(err);
            }

            PrimitiveArg::from_str(segment.ident.to_string().as_str()).or(Err(err))
        } else {
            Err(err)
        }
    }
}

#[derive(Debug, Clone)]
struct InstructionArgument {
    name: String,
    ty: PrimitiveArg,
    description: String,
}

impl Parse for InstructionArgument {
    fn parse(input: ParseStream) -> Result<Self> {
        let ident: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let ty: Type = input.parse()?;

        // Optional: a single `key = value` pair as `desc = "argument description"`.
        let mut description: String = "".to_string();

        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            match input.parse::<Lit>() {
                Ok(Lit::Str(s)) => description = s.value(),
                _ => return Err(ParsingError::ExpectedArgumentDescription.into_err(input.span())),
            }
        }

        Ok(InstructionArgument {
            name: ident.to_string(),
            ty: PrimitiveArg::try_from(&ty)?,
            description,
        })
    }
}

#[derive(Debug, Clone)]
struct InstructionAccount {
    index: u8,
    is_writable: bool,
    is_signer: bool,
    name: String,
    description: String,
}

enum ParsingError {
    NotAnEnum,
    EnumVariantShouldBeFieldless,
    ZeroAccounts,
    MissingSigner,
    DuplicateName(String, String),
    AccountNeedsIndexAndName,
    UnexpectedAttribute(String),
    InvalidIndexU8(String),
    TooManyIndices(u8, u8),
    TooManyNames(String, String),
    TooManyDescriptions,
    ExpectedNameValueLiteral(String),
    IndexOutOfOrder(u8, usize),
    InvalidPrimitiveType,
    ExpectedArgumentDescription,
    InvalidLiteralU8,
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        use strum::IntoEnumIterator;

        match value {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
            ParsingError::EnumVariantShouldBeFieldless => {
                "Enum variants should be fieldless".into()
            }
            ParsingError::ZeroAccounts => "Instruction has no accounts".into(),
            ParsingError::MissingSigner => "Instruction must have at least one signer".into(),
            ParsingError::DuplicateName(dupe_type, name) => {
                format!("Duplicate {dupe_type} name: {name}")
            }
            ParsingError::AccountNeedsIndexAndName => "Accounts need an index and a name".into(),
            ParsingError::UnexpectedAttribute(attr) => format!("Unexpected attribute: {attr}"),
            ParsingError::InvalidIndexU8(index) => format!("Invalid u8 index: {index}"),
            ParsingError::ExpectedNameValueLiteral(value) => {
                format!("Expected name = \"value\" literal, got: {value}")
            }
            ParsingError::TooManyDescriptions => "Account has too many descriptions".into(),
            ParsingError::TooManyNames(a, b) => format!("Account has too many names: {a}, {b}"),
            ParsingError::TooManyIndices(a, b) => format!("Account has too many indices: {a}, {b}"),
            ParsingError::IndexOutOfOrder(idx, pos) => {
                format!("Account index {idx} doesn't match position {pos}")
            }
            ParsingError::InvalidPrimitiveType => format!(
                "Invalid argument type, valid types include: {}",
                PrimitiveArg::iter().join(", ")
            ),
            ParsingError::ExpectedArgumentDescription => {
                "Expected a string literal for the argument description".into()
            }
            ParsingError::InvalidLiteralU8 => "Enum variant must be a literal u8".into(),
        }
    }
}

impl ParsingError {
    #[inline]
    pub fn into_err(self, span: proc_macro2::Span) -> syn::Error {
        Error::new::<String>(span, self.into())
    }
}

impl TryFrom<(usize, &Attribute)> for InstructionAccount {
    type Error = Error;

    fn try_from(pos_and_attr: (usize, &Attribute)) -> std::result::Result<Self, Self::Error> {
        let (position, attribute) = pos_and_attr;
        let span = attribute.meta.span();
        let list = &attribute.meta.require_list()?;

        let (index, is_writable, is_signer, name, description) =
            list.parse_args_with(build_instruction_account)?;

        // Ensure the index and name were set.
        let (index, name) = match (index, name) {
            (Some(index), Some(name)) => Ok((index, name)),
            _ => Err(ParsingError::AccountNeedsIndexAndName.into_err(span)),
        }?;

        if position != index as usize {
            return Err(ParsingError::IndexOutOfOrder(index, position).into_err(span));
        }

        Ok(InstructionAccount {
            index,
            is_writable,
            is_signer,
            name,
            description,
        })
    }
}

type InstructionAccountInConstruction = (Option<u8>, bool, bool, Option<String>, String);

fn build_instruction_account(input: ParseStream) -> Result<InstructionAccountInConstruction> {
    // Build the InstructionAccount by setting each field as it's encountered.
    let (mut index, mut is_writable, mut is_signer, mut name, mut description) =
        (None, false, false, None, None);

    while !input.is_empty() {
        match input {
            // Consume the account's u8 index.
            buf if input.peek(LitInt) => {
                let lit: LitInt = buf.parse()?;
                let val: u8 =
                    lit.base10_parse().or(Err(
                        ParsingError::InvalidIndexU8(lit.to_string()).into_err(buf.span())
                    ))?;
                if let Some(existing) = index {
                    return Err(ParsingError::TooManyIndices(existing, val).into_err(buf.span()));
                }
                index.replace(val);
            }
            // Consume commas.
            comma if input.peek(Token![,]) => {
                let _: Token![,] = comma.parse()?;
            }
            // Consume the remaining positional, named fields.
            meta_input => {
                let m: Meta = meta_input
                    .parse()
                    .or(Err(ParsingError::UnexpectedAttribute(
                        meta_input.to_string(),
                    )
                    .into_err(meta_input.span())))?;
                if m.path().is_ident(ACCOUNT_SIGNER) {
                    is_signer = true;
                } else if m.path().is_ident(ACCOUNT_WRITABLE) {
                    is_writable = true;
                } else if m.path().is_ident(ACCOUNT_NAME) {
                    let name_str = parse_name_value(&m)?;
                    if let Some(old) = name {
                        return Err(ParsingError::TooManyNames(old, name_str).into_err(m.span()));
                    }
                    name.replace(name_str);
                } else if m.path().is_ident(DESCRIPTION) {
                    let new_description = parse_name_value(&m)?;
                    if description.is_some() {
                        return Err(ParsingError::TooManyDescriptions.into_err(m.span()));
                    }
                    description.replace(new_description);
                } else {
                    let unexpected = m
                        .path()
                        .get_ident()
                        .map(|v| v.to_token_stream())
                        .unwrap_or(m.path().to_token_stream())
                        .to_string();
                    return Err(ParsingError::UnexpectedAttribute(unexpected).into_err(m.span()));
                }
            }
        }
    }

    Ok((
        index,
        is_writable,
        is_signer,
        name,
        description.unwrap_or_default(),
    ))
}

/// Parses a Meta as a `name = "value"` meta, expecting the right-hand expr to be a string literal.
fn parse_name_value(meta: &Meta) -> std::result::Result<String, Error> {
    let expr = &meta.require_name_value()?.value;
    if let Expr::Lit(syn::ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = expr
    {
        Ok(lit_str.value())
    } else {
        let got = expr.to_token_stream().to_string();
        Err(ParsingError::ExpectedNameValueLiteral(got).into_err(meta.span()))
    }
}

/// Validate the vector of instruction accounts to ensure no duplicate names, indices, etc.
fn validate_accounts(accs: Vec<InstructionAccount>, span: proc_macro2::Span) -> Result<()> {
    if accs.is_empty() {
        return Err(ParsingError::ZeroAccounts.into_err(span));
    }

    if !accs.iter().any(|acc| acc.is_signer) {
        return Err(ParsingError::MissingSigner.into_err(span));
    }

    let names: Vec<String> = accs.iter().map(|acc| acc.name.clone()).collect();
    check_duplicate_names(names, span, "account")?;

    Ok(())
}

/// Validate the vector of instruction arguments to ensure no duplicate names.
fn validate_args(args: Vec<InstructionArgument>, span: proc_macro2::Span) -> Result<()> {
    let names: Vec<String> = args.iter().map(|arg| arg.name.clone()).collect();
    check_duplicate_names(names, span, "argument")
}

fn check_duplicate_names(
    mut names: Vec<String>,
    span: proc_macro2::Span,
    dupe_type: &str,
) -> Result<()> {
    names.sort();
    names
        .windows(2)
        .map(|window| <&[String; 2]>::try_from(window).expect("Should have 2"))
        .try_for_each(|[prev_name, curr_name]| {
            if prev_name == curr_name {
                let e = ParsingError::DuplicateName(dupe_type.to_string(), curr_name.clone());
                Err(e.into_err(span))
            } else {
                Ok(())
            }
        })
}
