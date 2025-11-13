//! Utilities for recognizing and handling primitive argument types used in instruction argument
//! attributes.
//!
//! Instruction arguments only support argument types defined in [`PrimitiveArg`].

use std::str::FromStr;

use syn::Type;

use crate::parse::{
    argument_type::ParsedPackableType,
    parsing_error::ParsingError,
};

/// An enum for all of the argument types recognized by the instruction argument attribute.
#[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum PrimitiveArg {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl ParsedPackableType for PrimitiveArg {
    fn size(&self) -> usize {
        match self {
            Self::U8 => size_of::<u8>(),
            Self::U16 => size_of::<u16>(),
            Self::U32 => size_of::<u32>(),
            Self::U64 => size_of::<u64>(),
            Self::U128 => size_of::<u128>(),
        }
    }

    fn as_parsed_type(&self) -> Type {
        syn::parse_str(&self.to_string()).expect("All types should be valid")
    }
}

impl From<PrimitiveArg> for Type {
    fn from(value: PrimitiveArg) -> Self {
        value.as_parsed_type()
    }
}

impl TryFrom<&Type> for PrimitiveArg {
    type Error = syn::Error;

    fn try_from(ty: &Type) -> std::result::Result<Self, Self::Error> {
        let err = ParsingError::InvalidArgumentType.new_err(ty);
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

            // Try converting the segment identifier to a stringified `PrimitiveArg` variant.
            PrimitiveArg::from_str(&segment.ident.to_string()).or(Err(err))
        } else {
            Err(err)
        }
    }
}
