use syn::{
    spanned::Spanned,
    DataEnum,
    DeriveInput,
};

use crate::{
    parsing_error,
    ParsingError,
};

pub fn require_data_enum(input: DeriveInput) -> syn::Result<DataEnum> {
    match input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => Err(parsing_error!(input, ParsingError::NotAnEnum)),
    }
}
