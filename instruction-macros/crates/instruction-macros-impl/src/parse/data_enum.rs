use syn::{
    spanned::Spanned,
    DataEnum,
    DeriveInput,
};

use crate::{
    parsing_bail,
    ParsingError,
};

pub fn require_data_enum(input: DeriveInput) -> syn::Result<DataEnum> {
    match input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => parsing_bail!(input, ParsingError::NotAnEnum),
    }
}
