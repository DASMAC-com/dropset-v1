use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DropsetError {
    InvalidInstructionTag,
    InsufficientByteLength,
    InvalidSectorIndex,
    NoFreeNodesLeft,
    InvalidAccountDiscriminant,
    IndexOutOfBounds,
    InvalidIndexHint,
    UnalignedData,
    UnallocatedAccountData,
    UserAlreadyExists,
    NotEnoughAccountKeys,
    InvalidTokenProgram,
    AlreadyInitializedAccount,
    NotOwnedBySystemProgram,
    AddressDerivationFailed,
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
            DropsetError::InvalidSectorIndex => "Invalid sector index passed",
            DropsetError::NoFreeNodesLeft => "There are no free stack nodes left",
            DropsetError::InvalidAccountDiscriminant => "Invalid account discriminant",
            DropsetError::IndexOutOfBounds => "Index is out of bounds",
            DropsetError::InvalidIndexHint => "Index hint is invalid",
            DropsetError::UnalignedData => "Account data is unaligned",
            DropsetError::UnallocatedAccountData => "Account data hasn't been properly allocated",
            DropsetError::UserAlreadyExists => "User already has an existing seat",
            DropsetError::NotEnoughAccountKeys => "Not enough account keys were provided",
            DropsetError::InvalidTokenProgram => "Invalid token program ID",
            DropsetError::AlreadyInitializedAccount => "Account has already been initialized",
            DropsetError::NotOwnedBySystemProgram => "Account is not owned by the system program",
            DropsetError::AddressDerivationFailed => "PDA derivation failed",
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl core::fmt::Display for DropsetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type DropsetResult = Result<(), DropsetError>;
