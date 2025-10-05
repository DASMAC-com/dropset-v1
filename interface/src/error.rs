use pinocchio::program_error::ProgramError;

#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DropsetError {
    InvalidInstructionTag,
    InsufficientByteLength,
    UninitializedData,
    InvalidAccountDiscriminant,
    UnallocatedAccountData,
    UnalignedData,
    InvalidSectorIndex,
    IndexOutOfBounds,
    NoFreeNodesLeft,
    InvalidPackedDataLength,
    InvalidIndexHint,
    IncorrectDropsetProgram,
    IncorrectSystemProgram,
    InvalidTokenProgram,
    OwnerNotTokenProgram,
    MintInfoMismatch,
    AlreadyInitializedAccount,
    NotOwnedBySystemProgram,
    IncorrectTokenAccountOwner,
    NotEnoughAccountKeys,
    InvalidMarketAccountOwner,
    InvalidMintAccount,
    InvalidNonZeroInteger,
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
            DropsetError::UnalignedData => "Account data is unaligned",
            DropsetError::InvalidSectorIndex => "Invalid sector index passed",
            DropsetError::IndexOutOfBounds => "Index out of bounds",
            DropsetError::NoFreeNodesLeft => "There are no free stack nodes left",
            DropsetError::InvalidPackedDataLength => "Invalid packed data length",
            DropsetError::InvalidIndexHint => "Invalid index hint",
            DropsetError::IncorrectDropsetProgram => "Incorrect dropset program ID",
            DropsetError::IncorrectSystemProgram => "Incorrect system program ID",
            DropsetError::InvalidTokenProgram => "Invalid token program ID",
            DropsetError::OwnerNotTokenProgram => "Account owner must be a valid token program",
            DropsetError::MintInfoMismatch => "Mint info does not match",
            DropsetError::AlreadyInitializedAccount => "Account has already been initialized",
            DropsetError::NotOwnedBySystemProgram => "Account is not owned by the system program",
            DropsetError::IncorrectTokenAccountOwner => "Incorrect associated token account owner",
            DropsetError::NotEnoughAccountKeys => "Not enough account keys were provided",
            DropsetError::InvalidMarketAccountOwner => "Invalid market account owner",
            DropsetError::InvalidMintAccount => "Invalid mint account",
            DropsetError::InvalidNonZeroInteger => "Value passed must be greater than zero",
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
