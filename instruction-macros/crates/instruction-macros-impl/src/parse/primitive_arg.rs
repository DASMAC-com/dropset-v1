use std::str::FromStr;

use syn::Type;

use crate::parse::parsing_error::ParsingError;

#[derive(Debug, Clone, strum_macros::EnumIter, strum_macros::Display, strum_macros::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum PrimitiveArg {
    U8,
    U16,
    U32,
    U64,
    U128,
}

impl PrimitiveArg {
    pub const fn size(&self) -> usize {
        match self {
            Self::U8 => size_of::<u8>(),
            Self::U16 => size_of::<u16>(),
            Self::U32 => size_of::<u32>(),
            Self::U64 => size_of::<u64>(),
            Self::U128 => size_of::<u128>(),
        }
    }

    pub fn as_parsed_type(&self) -> syn::Type {
        syn::parse_str(&self.to_string()).expect("All types should be valid")
    }
}

impl TryFrom<&Type> for PrimitiveArg {
    type Error = syn::Error;

    fn try_from(ty: &Type) -> std::result::Result<Self, Self::Error> {
        let err = ParsingError::InvalidPrimitiveType.new_err(ty);
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
