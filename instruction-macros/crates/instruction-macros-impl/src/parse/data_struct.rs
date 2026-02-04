//! See [`require_data_struct`].

use syn::{
    DataStruct,
    DeriveInput,
};

use crate::ParsingError;

/// Ensures the macro input is a [`DataStruct`] and returns it or a typed error.
pub fn require_data_struct(input: DeriveInput) -> syn::Result<DataStruct> {
    match input.data {
        syn::Data::Struct(ds) => Ok(ds),
        _ => Err(ParsingError::NotAStruct.new_err(input)),
    }
}
