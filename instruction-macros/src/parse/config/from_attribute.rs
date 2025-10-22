use syn::{
    spanned::Spanned,
    Attribute,
};

use crate::{
    parse::config::error_type::ErrorType,
    ParsingError,
};

const DEFAULT_ERROR_BASE: &str = "ProgramError";
const DEFAULT_TAG_ERROR_VARIANT: &str = "InvalidInstructionData";
const DEFAULT_NUM_ACCOUNTS_ERROR_VARIANT: &str = "NotEnoughAccountKeys";
const DEFAULT_INVALID_INSTRUCTION_DATA: &str = "InvalidInstructionData";
const INVALID_TAG_ATTR: &str = "error_invalid_tag";
const INVALID_NUM_ACCOUNTS_ATTR: &str = "error_invalid_num_accounts";
const INVALID_INSTRUCTION_DATA_ATTR: &str = "error_invalid_instruction_data";

pub const VALID_CONFIG_ATTRIBUTES: [&str; 3] = [
    INVALID_TAG_ATTR,
    INVALID_NUM_ACCOUNTS_ATTR,
    INVALID_INSTRUCTION_DATA_ATTR,
];

#[derive(Debug)]
pub struct Config {
    pub errors: ErrorTypes,
}

#[derive(Debug)]
pub struct ErrorTypes {
    /// The error type for `TryFrom<u8>` for the instruction tag.
    pub invalid_tag: ErrorType,
    /// The error type for failing to destructure an `&[AccountInfo]` slice into named fields.
    pub invalid_num_accounts: ErrorType,
    /// The error type for failing to unpack a slice `instruction_data: &[u8]` to a data variant.
    pub invalid_instruction_data: ErrorType,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            errors: ErrorTypes {
                invalid_tag: ErrorType::new(DEFAULT_ERROR_BASE, DEFAULT_TAG_ERROR_VARIANT),
                invalid_num_accounts: ErrorType::new(
                    DEFAULT_ERROR_BASE,
                    DEFAULT_NUM_ACCOUNTS_ERROR_VARIANT,
                ),
                invalid_instruction_data: ErrorType::new(
                    DEFAULT_ERROR_BASE,
                    DEFAULT_INVALID_INSTRUCTION_DATA,
                ),
            },
        }
    }
}

impl TryFrom<&Attribute> for Config {
    type Error = syn::Error;

    fn try_from(attr: &Attribute) -> std::result::Result<Self, Self::Error> {
        let mut res = Config::default();

        attr.parse_nested_meta(|meta| {
            let invalid_err_type = || ParsingError::InvalidErrorType.into_err(meta.path.span());

            if let Some(ident) = meta.path.get_ident().map(|v| v.to_string()) {
                match ident.as_str() {
                    INVALID_NUM_ACCOUNTS_ATTR => {
                        let err_ty: ErrorType =
                            meta.value().map_err(|_| invalid_err_type())?.parse()?;
                        res.errors.invalid_num_accounts = err_ty;
                        Ok(())
                    }
                    INVALID_TAG_ATTR => {
                        let err_ty: ErrorType =
                            meta.value().map_err(|_| invalid_err_type())?.parse()?;
                        res.errors.invalid_tag = err_ty;
                        Ok(())
                    }
                    INVALID_INSTRUCTION_DATA_ATTR => {
                        let err_ty: ErrorType =
                            meta.value().map_err(|_| invalid_err_type())?.parse()?;
                        res.errors.invalid_instruction_data = err_ty;
                        Ok(())
                    }
                    _ => Err(ParsingError::InvalidConfigAttribute.into_err(ident.span())),
                }
            } else {
                Ok(())
            }
        })?;

        Ok(res)
    }
}
