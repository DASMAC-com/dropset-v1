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
    AccountNotInitialized,
    NotOwnedBySystemProgram,
    AddressDerivationFailed,
    AmountCannotBeZero,
    InsufficientUserBalance,
    OwnerNotTokenProgram,
    MintInfoMismatch,
    IncorrectTokenAccountOwner,
    InvalidMintAccount,
    InvalidMarketAccountOwner,
    MissingIndexHint,
    InvalidNonZeroInteger,
    InvalidInstructionData,
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
            DropsetError::AccountNotInitialized => "Account hasn't been initialized",
            DropsetError::NotOwnedBySystemProgram => "Account is not owned by the system program",
            DropsetError::AddressDerivationFailed => "PDA derivation failed",
            DropsetError::AmountCannotBeZero => "Amount can't be zero",
            DropsetError::InsufficientUserBalance => "Insufficient user balance",
            DropsetError::OwnerNotTokenProgram => "Account owner must be a valid token program",
            DropsetError::MintInfoMismatch => "Mint info does not match",
            DropsetError::IncorrectTokenAccountOwner => "Incorrect associated token account owner",
            DropsetError::InvalidMintAccount => "Invalid mint account",
            DropsetError::InvalidMarketAccountOwner => "Invalid market account owner",
            DropsetError::MissingIndexHint => "Instruction data must include an index hint",
            DropsetError::InvalidNonZeroInteger => "Value passed must be greater than zero",
            DropsetError::InvalidInstructionData => "Instruction data is invalid",
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
