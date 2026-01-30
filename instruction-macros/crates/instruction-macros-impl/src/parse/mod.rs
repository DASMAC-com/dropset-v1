//! Parsing utilities for the instruction macros crate. These utilites parse the instruction enum,
//! attributes, and metadata into validated intermediate structures.

pub mod argument_type;
pub mod data_enum;
pub mod data_struct;
pub mod error_path;
pub mod error_type;
pub mod instruction_account;
pub mod instruction_argument;
pub mod instruction_discriminant;
pub mod instruction_variant;
pub mod name_value;
pub mod parsed_enum;
pub mod parsed_struct;
pub mod parsing_error;
pub mod primitive_arg;
pub mod program_id;
pub mod require_repr;
pub mod validation;
