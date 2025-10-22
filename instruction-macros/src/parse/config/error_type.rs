use syn::{
    parse::Parse,
    spanned::Spanned,
    Ident,
    Path,
    PathSegment,
};

use crate::ParsingError;

#[derive(Debug)]
pub struct ErrorType {
    /// The error base segment; e.g. `ProgramError`
    pub base: Path,
    /// The error variant; e.g. `InvalidInstructionData`
    pub variant: Ident,
    /// The fully qualified path; e.g. `ProgramError::InvalidInstructionData`
    pub full_path: Path,
}

impl ErrorType {
    pub fn new(base_str: &str, variant_str: &str) -> Self {
        let base = syn::parse_str::<Path>(base_str).expect("Invalid base path");
        let variant = syn::parse_str::<Ident>(variant_str).expect("Invalid variant ident");

        let mut full_path = base.clone();
        assert!(
            !base.segments.empty_or_trailing(),
            "Invalid base segment for error type"
        );
        full_path.segments.push(PathSegment {
            ident: variant.clone(),
            arguments: syn::PathArguments::None,
        });
        ErrorType {
            base,
            variant,
            full_path,
        }
    }
}

impl Parse for ErrorType {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let invalid_err_type = || ParsingError::InvalidErrorType.into_err(input.span());

        let full_path: Path = input.parse().map_err(|_| invalid_err_type())?;

        if full_path.segments.len() < 2 {
            return Err(ParsingError::ErrorNotFullyQualified.into_err(full_path.span()));
        };

        let mut base = full_path.clone();
        let variant = base
            .segments
            .pop()
            .ok_or(invalid_err_type())?
            .into_value()
            .ident;
        base.segments.pop_punct();

        Ok(Self {
            base,
            variant,
            full_path,
        })
    }
}
