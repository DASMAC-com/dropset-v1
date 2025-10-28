use syn::{
    spanned::Spanned,
    DataEnum,
    DeriveInput,
};

use crate::{
    parsing_err,
    ParsingError,
};

pub fn require_data_enum(input: DeriveInput) -> syn::Result<DataEnum> {
    match input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => parsing_err!(input, ParsingError::NotAnEnum),
    }
}
