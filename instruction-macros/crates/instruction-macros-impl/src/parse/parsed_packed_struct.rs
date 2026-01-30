use syn::{
    DataStruct,
    DeriveInput,
    Ident,
};

use crate::parse::data_struct::require_data_struct;

/// The validated, in-memory model of the data struct used by parsing and rendering functions.
pub struct ParsedStruct {
    pub struct_ident: Ident,
    pub data_struct: DataStruct,
}

impl ParsedStruct {
    pub fn new(input: DeriveInput) -> Result<Self, syn::Error> {
        let enum_ident = input.ident.clone();
        let data_struct = require_data_struct(input)?;

        Ok(Self {
            struct_ident: enum_ident,
            data_struct,
        })
    }
}
