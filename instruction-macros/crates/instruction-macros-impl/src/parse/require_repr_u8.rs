use syn::{
    punctuated::Punctuated,
    DeriveInput,
    Meta,
    Token,
};

use crate::parse::parsing_error::ParsingError;

const REPR_IDENT: &str = "repr";
const U8_IDENT: &str = "u8";

pub fn require_repr_u8(input: &DeriveInput) -> syn::Result<()> {
    for attr in &input.attrs {
        if !attr.path().is_ident(REPR_IDENT) {
            continue;
        }

        let nested_meta = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        if nested_meta
            .iter()
            .any(|meta| meta.path().is_ident(U8_IDENT))
        {
            return Ok(());
        }
    }

    Err(ParsingError::ExpectedReprU8.new_err(&input.ident))
}
