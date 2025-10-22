use syn::{
    DataEnum,
    DeriveInput,
    Ident,
};

use crate::parse::data_enum::parse_data_enum;

pub struct ParsedEnum {
    pub enum_ident: Ident,
    pub data_enum: DataEnum,
}

impl TryFrom<DeriveInput> for ParsedEnum {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let enum_ident = input.ident.clone();
        let data_enum = parse_data_enum(input)?;

        Ok(Self {
            data_enum,
            enum_ident,
        })
    }
}
