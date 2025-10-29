pub enum ParsingError {
    NotAnEnum,
    ProgramIdMissing,
    InvalidProgramIdPath,
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        match value {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
            ParsingError::ProgramIdMissing =>
                "Program ID not found. Specify the `[u8; 32]` program ID path like so: `#[program_id(program::ID)]`".into(),
            ParsingError::InvalidProgramIdPath =>
                "Program ID path must start with `crate::`, `::`, or be a single local identifier like `PROGRAM_ID`".into(),
}
    }
}

impl ParsingError {
    #[inline]
    pub fn new_err(self, span: impl syn::spanned::Spanned) -> syn::Error {
        syn::Error::new::<String>(span.span(), self.into())
    }
}
