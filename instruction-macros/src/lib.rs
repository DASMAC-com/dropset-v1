use std::str::FromStr;

use itertools::Itertools;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Attribute, DeriveInput, Error, Expr, Ident, Lit, LitInt, Meta, Result, Token, Type,
};

const ACCOUNT_IDENTIFIER: &str = "account";
const ACCOUNT_NAME: &str = "name";
const ACCOUNT_DESCRIPTION: &str = "desc";
const ACCOUNT_WRITABLE: &str = "writable";
const ACCOUNT_SIGNER: &str = "signer";
const ARGUMENT_IDENTIFIER: &str = "args";

#[proc_macro_derive(ProgramInstructions, attributes(account, args))]
pub fn instruction(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let out = impl_instruction(input).unwrap_or_else(|e| e.to_compile_error());
    out.into()
}

fn impl_instruction(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_item = match input.data {
        syn::Data::Enum(e) => e,
        _ => return Err(ParsingError::NotAnEnum.into_err(input.span())),
    };

    // For each enum variant, check all attrs.
    enum_item.variants.into_iter().try_for_each(|variant| {
        // `ident` here is the name of the enum variant
        let variant_name = &variant.ident;
        eprintln!("{variant_name}");

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
            .map(InstructionArgument::try_from)
            .collect::<Result<Vec<InstructionArgument>>>()?;

        instruction_arguments.iter().for_each(|arg| {
            eprintln!("{:?}", arg);
        });

        eprintln!("{:#?}", instruction_accounts);

        validate_accounts(instruction_accounts, variant.span())?;
        validate_args(instruction_arguments, variant.span())?;

        Ok::<(), Error>(())
    })?;

    Ok(proc_macro2::TokenStream::new())
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

impl TryFrom<Ident> for PrimitiveArg {
    type Error = Error;

    fn try_from(value: Ident) -> std::result::Result<Self, Self::Error> {
        PrimitiveArg::from_str(value.to_string().as_str()).or(Err(
            ParsingError::InvalidPrimitiveType.into_err(value.span()),
        ))
    }
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
}

/// An argument pair of the exact form (ident: type) e.g. (foo: u64).
struct ArgPair {
    ident: Ident,
    _colon: Token![:],
    ty: Type,
}

impl Parse for ArgPair {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _colon: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl TryFrom<&Attribute> for InstructionArgument {
    type Error = Error;

    fn try_from(attr: &Attribute) -> std::result::Result<Self, Self::Error> {
        let tokens: ArgPair = attr.parse_args()?;
        Ok(InstructionArgument {
            name: tokens.ident.to_string(),
            ty: PrimitiveArg::try_from(&tokens.ty)?,
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
    TooManyArgumentAttributes,
    InvalidPrimitiveType,
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        use strum::IntoEnumIterator;

        match value {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
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
            ParsingError::TooManyArgumentAttributes => {
                "Too many argument attributes, expected only one".into()
            }
            ParsingError::InvalidPrimitiveType => format!(
                "Invalid argument type, valid types include: {}",
                PrimitiveArg::iter().join(", ")
            ),
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
                } else if m.path().is_ident(ACCOUNT_DESCRIPTION) {
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
