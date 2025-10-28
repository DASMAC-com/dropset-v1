use syn::{
    DataEnum,
    DeriveInput,
};

use crate::ParsingError;

pub fn require_data_enum(input: DeriveInput) -> syn::Result<DataEnum> {
    match input.data {
        syn::Data::Enum(e) => Ok(e),
        _ => Err(ParsingError::NotAnEnum.new_err(input)),
    }
}
