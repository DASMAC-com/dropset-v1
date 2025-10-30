use syn::{
    DataEnum,
    DeriveInput,
    Ident,
    Path,
};

use crate::parse::{
    data_enum::require_data_enum,
    program_id::ProgramID,
    require_repr_u8::require_repr_u8,
};

pub struct ParsedEnum {
    pub enum_ident: Ident,
    pub data_enum: DataEnum,
    pub program_id_path: Path,
}

impl TryFrom<DeriveInput> for ParsedEnum {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let enum_ident = input.ident.clone();
        let program_id = ProgramID::try_from(&input)?;
        require_repr_u8(&input)?;
        let data_enum = require_data_enum(input)?;

        Ok(Self {
            data_enum,
            enum_ident,
            program_id_path: program_id.0,
        })
    }
}
