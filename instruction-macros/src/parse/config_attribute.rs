use syn::{
    spanned::Spanned,
    Attribute,
    ExprPath,
    Ident,
    Type,
};

use crate::{
    ParsingError,
    DEFAULT_TAG_ERROR_BASE,
    DEFAULT_TAG_ERROR_TYPE,
};

const CONFIG_ERROR_ATTR: &str = "error";

#[derive(Clone, Debug)]
pub struct Config {
    /// The error base segment; e.g. `ProgramError`
    pub error_base: Type,
    /// The error variant; e.g. `InvalidInstructionData`
    pub error_variant: Ident,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            error_base: syn::parse_str::<Type>(DEFAULT_TAG_ERROR_BASE).unwrap(),
            error_variant: syn::parse_str::<Ident>(DEFAULT_TAG_ERROR_TYPE).unwrap(),
        }
    }
}

impl TryFrom<&Attribute> for Config {
    type Error = syn::Error;

    fn try_from(attr: &Attribute) -> std::result::Result<Self, Self::Error> {
        let mut error_type_info = None::<(Type, Ident)>;

        let default = Config::default();

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(CONFIG_ERROR_ATTR) {
                let invalid_err_type = || ParsingError::InvalidErrorType.into_err(meta.path.span());
                let value = meta.value().map_err(|_| invalid_err_type())?;
                let expr_path: ExprPath = value.parse().map_err(|_| invalid_err_type())?;
                let path = &expr_path.path;
                if path.segments.len() < 2 {
                    return Err(ParsingError::ErrorNotFullyQualified.into_err(expr_path.span()));
                }

                let mut type_path = path.clone();
                // Get the type variant aka the specific type identifier: `InvalidInstructionData`.
                let last_portion = type_path
                    .segments
                    .pop()
                    .expect("Path segments should at least be len 2")
                    .into_value()
                    .ident;

                type_path.segments.pop_punct();

                // Get the remaining base: `ProgramError` in `ProgramError::InvalidInstructionData`.
                let base_portion = Type::Path(syn::TypePath {
                    qself: None,
                    path: type_path,
                });

                error_type_info = Some((base_portion, last_portion));
                Ok(())
            } else {
                Ok(())
            }
        })?;

        let (error_base, error_variant) = match error_type_info {
            Some(info) => info,
            None => (default.error_base, default.error_variant),
        };

        Ok(Self {
            error_base,
            error_variant,
        })
    }
}
