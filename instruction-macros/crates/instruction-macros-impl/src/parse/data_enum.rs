use syn::{
    spanned::Spanned,
    DataEnum,
    DeriveInput,
};

use crate::ParsingError;

pub fn require_data_enum(input: DeriveInput) -> syn::Result<DataEnum> {
    match input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => Err(ParsingError::NotAnEnum.into_err(input.span())),
    }
}
