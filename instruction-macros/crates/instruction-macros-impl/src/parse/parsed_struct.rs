use syn::{
    DeriveInput,
    Fields,
    Ident,
    Type,
};

use crate::parse::{
    data_struct::require_data_struct,
    parsing_error::ParsingError,
    require_repr::{
        require_repr,
        ReprType,
    },
};

/// The validated, parsed struct identifier and all field names, lengths, and offsets in the struct.
pub struct ParsedStruct {
    pub struct_ident: Ident,
    pub field_names: Vec<Ident>,
    pub field_types: Vec<Type>,
}

impl ParsedStruct {
    pub fn new(input: DeriveInput) -> Result<Self, syn::Error> {
        let struct_ident = input.ident.clone();
        require_repr(&input, ReprType::C)?;
        let data_struct = require_data_struct(input)?;

        let Fields::Named(fields) = data_struct.fields else {
            return Err(ParsingError::NotAStruct.new_err(data_struct.fields));
        };

        let (field_names, field_types) = fields
            .named
            .into_iter()
            .map(|field| (field.ident.expect("All fields should be named"), field.ty))
            .unzip();

        Ok(Self {
            struct_ident,
            field_names,
            field_types,
        })
    }
}
