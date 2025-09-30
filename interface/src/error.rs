use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DropsetError {
    InvalidInstructionTag,
    InsufficientByteLength,
    UninitializedData,
    InvalidAccountDiscriminant,
    UnallocatedAccountData,
}

impl From<DropsetError> for ProgramError {
    #[inline(always)]
    fn from(e: DropsetError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<DropsetError> for &'static str {
    fn from(value: DropsetError) -> Self {
        match value {
            DropsetError::InvalidInstructionTag => "Invalid instruction tag",
            DropsetError::InsufficientByteLength => "Not enough bytes passed",
            DropsetError::UninitializedData => "Data passed was not initialized",
            DropsetError::InvalidAccountDiscriminant => "Invalid account discriminant",
            DropsetError::UnallocatedAccountData => "Account data hasn't been properly allocated",
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl core::fmt::Display for DropsetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(not(target_os = "solana"))]
impl std::error::Error for DropsetError {}

pub type DropsetResult = Result<(), DropsetError>;
