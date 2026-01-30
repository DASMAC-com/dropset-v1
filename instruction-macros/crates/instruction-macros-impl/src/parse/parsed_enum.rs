//! Defines [`ParsedEnum`] and its parsing implementation, the overarching in-memory model for
//! Solana program instructions defined by the usage of macro attributes.

use syn::{
    DataEnum,
    DeriveInput,
    Ident,
    Path,
};

use super::require_repr::ReprType;
use crate::parse::{
    data_enum::require_data_enum,
    program_id::ProgramID,
    require_repr::require_repr,
};

/// The validated, in-memory model of the instruction enum used by parsing and rendering functions.
pub struct ParsedEnum {
    pub enum_ident: Ident,
    pub data_enum: DataEnum,
    pub program_id_path: Path,
    pub as_instruction_events: bool,
}

impl ParsedEnum {
    pub fn new(input: DeriveInput, as_instruction_events: bool) -> Result<Self, syn::Error> {
        let enum_ident = input.ident.clone();
        let program_id = ProgramID::try_from(&input)?;
        require_repr(&input, ReprType::U8)?;
        let data_enum = require_data_enum(input)?;

        Ok(Self {
            data_enum,
            enum_ident,
            program_id_path: program_id.0,
            as_instruction_events,
        })
    }
}
