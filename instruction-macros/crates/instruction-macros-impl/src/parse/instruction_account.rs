use quote::ToTokens;
use syn::{
    parse::ParseStream,
    spanned::Spanned,
    Attribute,
    LitInt,
    Meta,
    Token,
};

use crate::{
    parse::name_value::parse_name_value_literal,
    ParsingError,
    ACCOUNT_NAME,
    ACCOUNT_SIGNER,
    ACCOUNT_WRITABLE,
    DESCRIPTION,
};

#[derive(Debug, Clone)]
pub struct InstructionAccount {
    pub index: u8,
    pub is_writable: bool,
    pub is_signer: bool,
    pub name: String,
    pub description: String,
}

impl TryFrom<(usize, &Attribute)> for InstructionAccount {
    type Error = syn::Error;

    fn try_from(pos_and_attr: (usize, &Attribute)) -> std::result::Result<Self, Self::Error> {
        let (position, attribute) = pos_and_attr;
        let span = attribute.meta.span();
        let list = &attribute.meta.require_list()?;

        let (index, is_writable, is_signer, name, description) =
            list.parse_args_with(build_instruction_account)?;

        // Ensure the index and name were set.
        let (index, name) = match (index, name) {
            (Some(index), Some(name)) => Ok((index, name)),
            _ => Err(ParsingError::AccountNeedsIndexAndName.new_err(span)),
        }?;

        if position != index as usize {
            return Err(ParsingError::IndexOutOfOrder(index, position).new_err(span));
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

fn build_instruction_account(input: ParseStream) -> syn::Result<InstructionAccountInConstruction> {
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
                        ParsingError::InvalidIndexU8(lit.to_string()).new_err(buf.span())
                    ))?;
                if let Some(existing) = index {
                    return Err(ParsingError::TooManyIndices(existing, val).new_err(buf.span()));
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
                    .new_err(meta_input.span())))?;
                if m.path().is_ident(ACCOUNT_SIGNER) {
                    is_signer = true;
                } else if m.path().is_ident(ACCOUNT_WRITABLE) {
                    is_writable = true;
                } else if m.path().is_ident(ACCOUNT_NAME) {
                    let name_str = parse_name_value_literal(&m)?;
                    if let Some(old) = name {
                        return Err(ParsingError::TooManyNames(old, name_str).new_err(m.span()));
                    }
                    name.replace(name_str);
                } else if m.path().is_ident(DESCRIPTION) {
                    let new_description = parse_name_value_literal(&m)?;
                    if description.is_some() {
                        return Err(ParsingError::TooManyDescriptions.new_err(m.span()));
                    }
                    description.replace(new_description);
                } else {
                    let unexpected = m
                        .path()
                        .get_ident()
                        .map(|v| v.to_token_stream())
                        .unwrap_or(m.path().to_token_stream())
                        .to_string();
                    return Err(ParsingError::UnexpectedAttribute(unexpected).new_err(m.span()));
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
