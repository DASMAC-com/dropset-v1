pub enum ParsingError {
    NotAnEnum,
    ProgramIdMissing,
    InvalidProgramIdPath,
}

impl ParsingError {
    #[inline]
    fn message(&self) -> String {
        match self {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
            ParsingError::ProgramIdMissing => "Program ID not found. Specify the `[u8; 32]` program ID path like so: `#[program_id(program::ID)]`".into(),
            ParsingError::InvalidProgramIdPath => "Program ID path must start with `crate::`, `::`, or be a single local identifier like `PROGRAM_ID`".into(),
        }
    }
}

impl std::fmt::Display for ParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message().as_str())
    }
}

#[macro_export]
/// Given a `span`-able value and an error type, create the `syn::Error`.
///
/// Example:
/// ```rust
/// use syn::spanned::Spanned;
///
/// let input: DeriveInput = parse_macro_input!(...);
///
/// if some_invalid_condition {
///   return Err(parsing_error!(input, ParsingError::InvalidInput));
/// }
/// ```
macro_rules! parsing_error {
    ( $span:expr, $err:expr ) => {
        syn::Error::new($span.span(), $err)
    };
}
