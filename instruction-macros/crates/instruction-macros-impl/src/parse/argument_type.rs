use std::fmt::Display;

use itertools::Itertools;
use quote::ToTokens;
use strum::IntoEnumIterator;
use syn::{
    parse::Parse,
    spanned::Spanned,
    token::Bracket,
    Type,
    TypeArray,
};

use crate::parse::{
    parsing_error::ParsingError,
    primitive_arg::PrimitiveArg,
};

#[derive(Debug, Clone)]
pub enum ArgumentType {
    PrimitiveArg(PrimitiveArg),
    PubkeyBytes,
}

impl ArgumentType {
    pub fn all_valid_types() -> String {
        format!("{}, {}", PrimitiveArg::iter().join(", "), PUBKEY_TYPE_STR)
    }
}

impl Parse for ArgumentType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Bracket) {
            let ty: TypeArray = input
                .parse()
                .map_err(|_| ParsingError::ExpectedPubkeyType.new_err(input.span()))?;
            let is_u8 = ty.elem.as_ref() == &PrimitiveArg::U8.into();
            let is_len_32 = ty.len == syn::parse_str("32").expect("Should be a valid path");

            if !is_u8 || !is_len_32 {
                return Err(ParsingError::InvalidPubkeyType(
                    ty.elem.to_token_stream().to_string(),
                    ty.len.to_token_stream().to_string(),
                )
                .new_err(ty.span()));
            }

            Ok(Self::PubkeyBytes)
        } else {
            let ty: Type = input
                .parse()
                .map_err(|_| ParsingError::InvalidArgumentType.new_err(input.span()))?;
            Ok(Self::PrimitiveArg(PrimitiveArg::try_from(&ty)?))
        }
    }
}

pub trait ParsedPackableType {
    /// Returns the byte size of the argument type.
    fn size(&self) -> usize;

    fn as_parsed_type(&self) -> Type;
}

const PUBKEY_BYTES: usize = 32;
const PUBKEY_TYPE_STR: &str = "[u8; 32]";

impl ParsedPackableType for ArgumentType {
    fn size(&self) -> usize {
        match self {
            Self::PubkeyBytes => PUBKEY_BYTES,
            Self::PrimitiveArg(arg) => arg.size(),
        }
    }

    fn as_parsed_type(&self) -> Type {
        match self {
            Self::PubkeyBytes => syn::parse_str(PUBKEY_TYPE_STR).expect("Should be a valid type"),
            Self::PrimitiveArg(arg) => arg.as_parsed_type(),
        }
    }
}

impl Display for ArgumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_parsed_type().to_token_stream())
    }
}
