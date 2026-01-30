//! See [`require_repr_u8`].

use syn::{
    punctuated::Punctuated,
    DeriveInput,
    Meta,
    Token,
};

use crate::parse::parsing_error::ParsingError;

const REPR_IDENT: &str = "repr";

#[derive(strum_macros::AsRefStr)]
pub enum ReprType {
    #[strum(serialize = "u8")]
    U8,
    C,
}

/// Ensures the instruction enum uses a `repr(<ReprType>)`-compatible layout and reports violations.
pub fn require_repr(input: &DeriveInput, repr_type: ReprType) -> syn::Result<()> {
    for attr in &input.attrs {
        if !attr.path().is_ident(REPR_IDENT) {
            continue;
        }

        let nested_meta = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        if nested_meta
            .iter()
            .any(|meta| meta.path().is_ident(repr_type.as_ref()))
        {
            return Ok(());
        }
    }

    match repr_type {
        ReprType::U8 => Err(ParsingError::ExpectedReprU8.new_err(&input.ident)),
        ReprType::C => Err(ParsingError::ExpectedReprC.new_err(&input.ident)),
    }
}
