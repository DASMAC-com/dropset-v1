//! Utilities for recognizing and handling [`PrimitiveArg`] types used in instruction argument
//! attributes.

use std::str::FromStr;

use syn::Type;

use crate::parse::argument_type::{
    ParsedPackableType,
    Size,
};

/// An enum for argument types with known sizes.
#[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum KnownType {
    #[strum(serialize = "bool")]
    Bool,
    #[strum(serialize = "u8")]
    U8,
    #[strum(serialize = "u16")]
    U16,
    #[strum(serialize = "u32")]
    U32,
    #[strum(serialize = "u64")]
    U64,
    #[strum(serialize = "u128")]
    U128,
    #[strum(serialize = "Address")]
    Address,
}

impl ParsedPackableType for KnownType {
    fn pack_len(&self) -> Size {
        match self {
            Self::Bool => Size::Lit(size_of::<bool>()),
            Self::U8 => Size::Lit(size_of::<u8>()),
            Self::U16 => Size::Lit(size_of::<u16>()),
            Self::U32 => Size::Lit(size_of::<u32>()),
            Self::U64 => Size::Lit(size_of::<u64>()),
            Self::U128 => Size::Lit(size_of::<u128>()),
            Self::Address => Size::Lit(size_of::<[u8; 32]>()),
        }
    }

    fn as_fully_qualified_type(&self) -> Type {
        let parsed = match self {
            Self::Address => syn::parse_str("::solana_address::Address"),
            _ => syn::parse_str(&self.to_string()),
        };

        parsed.expect("All types should be valid")
    }
}

impl From<KnownType> for Type {
    fn from(value: KnownType) -> Self {
        value.as_fully_qualified_type()
    }
}

impl KnownType {
    pub fn new(ty: Type) -> Option<Self> {
        let Type::Path(type_path) = ty else {
            return None;
        };

        // No qualified paths, only primitives.
        if type_path.qself.is_some() {
            return None;
        }
        // Only one segment, no `::` anywhere.
        if type_path.path.segments.len() != 1 {
            return None;
        }
        // No generics.
        let segment = &type_path.path.segments[0];
        if !segment.arguments.is_empty() {
            return None;
        }

        // Try converting the segment identifier to a known type from its string value.
        KnownType::from_str(&segment.ident.to_string()).ok()
    }
}
