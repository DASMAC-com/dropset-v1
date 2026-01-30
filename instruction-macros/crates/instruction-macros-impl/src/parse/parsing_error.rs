//! See [`ParsingError`].

use crate::parse::argument_type::ArgumentType;

/// Error types for reporting malformed macro inputs.
///
/// Can be converted to a clear, user-facing string message.
pub enum ParsingError {
    NotAnEnum,
    ProgramIdMissing,
    InvalidProgramIdPath,
    EnumVariantShouldBeFieldless,
    ZeroAccounts,
    MissingSigner,
    DuplicateName(String, String),
    AccountNeedsIndexAndName,
    UnexpectedAttribute(String),
    InvalidIndexU8(String),
    InvalidLiteralU8,
    TooManyIndices(u8, u8),
    TooManyNames(String, String),
    TooManyDescriptions,
    IndexOutOfOrder(u8, usize),
    InvalidArgumentType,
    ExpectedArgumentDescription,
    ExpectedNameValueLiteral(String),
    ExpectedReprU8,
    InstructionEventHasAccounts,
    NotAStruct,
    UnnamedFields,
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        match value {
            ParsingError::NotAnEnum => "This derive macro only works on enums".into(),
            ParsingError::ProgramIdMissing =>
                "Program ID not found. Specify the program ID path like so: `#[program_id(program::ID)]`".into(),
            ParsingError::InvalidProgramIdPath =>
                "Program ID path must start with `crate::`, `::`, or be a single local identifier like `PROGRAM_ID`".into(),
            ParsingError::EnumVariantShouldBeFieldless => "Enum variants should be fieldless".into(),
            ParsingError::ZeroAccounts => "Instruction has no accounts".into(),
            ParsingError::MissingSigner => "Instruction must have at least one signer".into(),
            ParsingError::DuplicateName(dupe_type, name) =>
                format!("Duplicate {dupe_type} name: {name}"),
            ParsingError::AccountNeedsIndexAndName => "Accounts need an index and a name".into(),
            ParsingError::UnexpectedAttribute(attr) => format!("Unexpected attribute: {attr}"),
            ParsingError::InvalidIndexU8(index) => format!("Invalid u8 index: {index}"),
            ParsingError::InvalidLiteralU8 => "Enum variant must be a literal u8".into(),
            ParsingError::TooManyDescriptions => "Account has too many descriptions".into(),
            ParsingError::TooManyNames(a, b) => format!("Account has too many names: {a}, {b}"),
            ParsingError::TooManyIndices(a, b) => format!("Account has too many indices: {a}, {b}"),
            ParsingError::IndexOutOfOrder(idx, pos) => format!("Account index {idx} doesn't match position {pos}"),
            ParsingError::InvalidArgumentType => format!(
                "Invalid argument type, valid types are: {}",
                ArgumentType::all_valid_types(),
            ),
            ParsingError::ExpectedArgumentDescription =>
                "Expected a string literal for the argument description".into(),
            ParsingError::ExpectedNameValueLiteral(value) =>
                format!("Expected name = \"value\" literal, got: {value}"),
            ParsingError::ExpectedReprU8 => "Enum does not have the attribute `#[repr(u8)]`".into(),
            ParsingError::InstructionEventHasAccounts => "Instruction event should not have any accounts".into(),
            ParsingError::NotAStruct => "This derive macro only works on structs".into(),
            ParsingError::UnnamedFields => "Packed struct must have named fields.".into(),
        }
    }
}

impl ParsingError {
    #[inline]
    pub fn new_err(self, span: impl syn::spanned::Spanned) -> syn::Error {
        syn::Error::new::<String>(span.span(), self.into())
    }
}
