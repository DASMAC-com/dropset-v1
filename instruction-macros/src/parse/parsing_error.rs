use itertools::Itertools;

use crate::parse::primitive_arg::PrimitiveArg;

pub(crate) enum ParsingError {
    NotAnEnum,
    EnumVariantShouldBeFieldless,
    ZeroAccounts,
    MissingSigner,
    DuplicateName(String, String),
    AccountNeedsIndexAndName,
    UnexpectedAttribute(String),
    InvalidIndexU8(String),
    TooManyIndices(u8, u8),
    TooManyNames(String, String),
    TooManyDescriptions,
    ExpectedNameValueLiteral(String),
    IndexOutOfOrder(u8, usize),
    InvalidPrimitiveType,
    ExpectedArgumentDescription,
    InvalidLiteralU8,
}

impl From<ParsingError> for String {
    #[inline]
    fn from(value: ParsingError) -> Self {
        use strum::IntoEnumIterator;

        match value {
            ParsingError::NotAnEnum => "Derive macro only works on enums".into(),
            ParsingError::EnumVariantShouldBeFieldless => {
                "Enum variants should be fieldless".into()
            }
            ParsingError::ZeroAccounts => "Instruction has no accounts".into(),
            ParsingError::MissingSigner => "Instruction must have at least one signer".into(),
            ParsingError::DuplicateName(dupe_type, name) => {
                format!("Duplicate {dupe_type} name: {name}")
            }
            ParsingError::AccountNeedsIndexAndName => "Accounts need an index and a name".into(),
            ParsingError::UnexpectedAttribute(attr) => format!("Unexpected attribute: {attr}"),
            ParsingError::InvalidIndexU8(index) => format!("Invalid u8 index: {index}"),
            ParsingError::ExpectedNameValueLiteral(value) => {
                format!("Expected name = \"value\" literal, got: {value}")
            }
            ParsingError::TooManyDescriptions => "Account has too many descriptions".into(),
            ParsingError::TooManyNames(a, b) => format!("Account has too many names: {a}, {b}"),
            ParsingError::TooManyIndices(a, b) => format!("Account has too many indices: {a}, {b}"),
            ParsingError::IndexOutOfOrder(idx, pos) => {
                format!("Account index {idx} doesn't match position {pos}")
            }
            ParsingError::InvalidPrimitiveType => format!(
                "Invalid argument type, valid types include: {}",
                PrimitiveArg::iter().join(", ")
            ),
            ParsingError::ExpectedArgumentDescription => {
                "Expected a string literal for the argument description".into()
            }
            ParsingError::InvalidLiteralU8 => "Enum variant must be a literal u8".into(),
        }
    }
}

impl ParsingError {
    #[inline]
    pub fn into_err(self, span: proc_macro2::Span) -> syn::Error {
        syn::Error::new::<String>(span, self.into())
    }
}
