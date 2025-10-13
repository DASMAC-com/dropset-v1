use quote::ToTokens;
use syn::{
    parse::ParseStream, parse_macro_input, spanned::Spanned, Attribute, DeriveInput, Error, Expr,
    Lit, LitInt, Meta, Result, Token,
};

#[proc_macro_derive(ProgramInstructions, attributes(account))]
pub fn instruction_accounts(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let out = impl_instruction_accounts(input).unwrap_or_else(|e| e.to_compile_error());
    out.into()
}

fn impl_instruction_accounts(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_item = match input.data {
        syn::Data::Enum(e) => e,
        _ => return Err(ParsingError::NotAnEnum.into_err(input.span())),
    };

    enum_item.variants.into_iter().try_for_each(|variant| {
        // `ident` here is the name of the enum variant
        let _variant_name = &variant.ident;

        // Filter by attrs matching `#[account(...)]`, then try converting to `InstructionAccount`s.
        let instruction_accounts = variant
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("account"))
            .enumerate()
            .map(InstructionAccount::try_from)
            .collect::<Result<Vec<InstructionAccount>>>()?;

        eprintln!("{:#?}", instruction_accounts);

        validate_and_sort_accounts(instruction_accounts, variant.span())?;

        Ok::<(), Error>(())
    })?;

    Ok(proc_macro2::TokenStream::new())
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
    DuplicateName(String),
    AccountNeedsIndexAndName,
    UnexpectedAttribute(String),
    InvalidIndexU8(String),
    TooManyIndices(u8, u8),
    TooManyNames(String, String),
    TooManyDescriptions,
    ExpectedNameValueLiteral(String),
    IndexOutOfOrder(u8, usize),
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        match value {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
            ParsingError::ZeroAccounts => "Instruction has no accounts".into(),
            ParsingError::MissingSigner => "Instruction must have at least one signer".into(),
            ParsingError::DuplicateName(name) => format!("Duplicate account name: {name}"),
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
                if m.path().is_ident("signer") {
                    is_signer = true;
                } else if m.path().is_ident("writable") {
                    is_writable = true;
                } else if m.path().is_ident("name") {
                    let name_str = parse_name_value(&m)?;
                    if let Some(old) = name {
                        return Err(ParsingError::TooManyNames(old, name_str).into_err(m.span()));
                    }
                    name.replace(name_str);
                } else if m.path().is_ident("desc") {
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
fn validate_and_sort_accounts(
    accs: Vec<InstructionAccount>,
    span: proc_macro2::Span,
) -> Result<()> {
    if accs.is_empty() {
        return Err(ParsingError::ZeroAccounts.into_err(span));
    }

    // Ensure there is at least one signer.
    if !accs.iter().any(|acc| acc.is_signer) {
        return Err(ParsingError::MissingSigner.into_err(span));
    }

    // Ensure there are no duplicate account names.
    let mut names: Vec<String> = accs.iter().map(|acc| acc.name.clone()).collect();
    names.sort();
    names
        .windows(2)
        .map(|window| <&[String; 2]>::try_from(window).expect("Should have 2"))
        .try_for_each(|[prev_name, curr_name]| {
            if prev_name == curr_name {
                Err(ParsingError::DuplicateName(curr_name.clone()).into_err(span))
            } else {
                Ok(())
            }
        })?;

    Ok(())
}
