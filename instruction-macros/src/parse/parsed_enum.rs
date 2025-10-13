use syn::{DataEnum, DeriveInput, Ident};

use crate::{
    parse::{config_attribute::Config, data_enum::parse_data_enum},
    CONFIG_ATTR,
};

pub struct ParsedEnum {
    pub enum_ident: Ident,
    pub data_enum: DataEnum,
    pub config: Config,
}

impl TryFrom<DeriveInput> for ParsedEnum {
    type Error = syn::Error;

    fn try_from(input: DeriveInput) -> Result<Self, Self::Error> {
        let config = input
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident(CONFIG_ATTR))
            .map(Config::try_from)
            .transpose()?
            .unwrap_or_default();

        let enum_ident = input.ident.clone();
        let data_enum = parse_data_enum(input)?;

        Ok(Self {
            config,
            data_enum,
            enum_ident,
        })
    }
}
