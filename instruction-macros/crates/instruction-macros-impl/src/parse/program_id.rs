use syn::{
    parse_quote,
    DeriveInput,
    Path,
};

use crate::{
    parse::parsing_error::ParsingError,
    PROGRAM_ID_IDENTIFIER,
};

/// An unambiguous path to a program ID.
///
/// ## Examples
/// ```rust
/// // With a leading colon.
/// program_id(::some_program::ID)
/// // With `crate::`
/// crate::program::ID
/// // As a standalone identifier.
/// PROGRAM_ID
///
/// // Invalid since it isn't definitively unambiguous.
/// program::ID
/// ```
pub struct ProgramID(pub syn::Path);

impl TryFrom<&DeriveInput> for ProgramID {
    type Error = syn::Error;

    fn try_from(input: &DeriveInput) -> Result<Self, Self::Error> {
        let err = || ParsingError::ProgramIdMissing.new_err(&input.ident);

        let program_id = input
            .attrs
            .iter()
            .find(|attr| {
                // Find a path of the form `#[my_path(...)]`
                // where `my_path` is the `ident`
                attr.meta
                    .path()
                    .get_ident()
                    .map(|v| v.to_string())
                    .unwrap_or_default()
                    == PROGRAM_ID_IDENTIFIER
            })
            .ok_or_else(err)?;

        let meta_list = program_id.meta.require_list().map_err(|_| err())?;

        // #[my_attr(value < 5)]
        //           ^^^^^^^^^ the args that get parsed
        let path: Path = meta_list.parse_args().map_err(|_| err())?;

        let program_id_path = match path {
            // e.g. `::some_absolute_path`
            p if p.leading_colon.is_some() => p,
            // e.g. `crate::some_absolute_path`
            p if p
                .segments
                .first()
                .map(|s| s.ident == "crate")
                .unwrap_or(false) =>
            {
                p
            }
            // e.g. `PROGRAM_ID`
            p if p.segments.len() == 1 => {
                let ident = p.segments.first().expect("len should be 1").ident.clone();
                let p: Path = parse_quote!(super::#ident);
                p
            }
            // Something invalid; e.g. `program::ID`, since that's ambiguous.
            p => return Err(ParsingError::InvalidProgramIdPath.new_err(p)),
        };

        Ok(ProgramID(program_id_path))
    }
}
