//! Common error types and conversion helpers to represent them as error message strings.

use pinocchio::error::ProgramError;
use price::OrderInfoError;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "client", derive(strum_macros::FromRepr))]
#[repr(u8)]
pub enum DropsetError {
    InvalidInstructionTag,
    InvalidInstructionEventTag,
    InsufficientByteLength,
    InvalidSectorIndex,
    NoFreeNodesLeft,
    InvalidAccountDiscriminant,
    IndexOutOfBounds,
    InvalidIndexHint,
    UnalignedData,
    UnallocatedAccountData,
    UserAlreadyExists,
    InvalidTokenProgram,
    AlreadyInitializedAccount,
    AccountNotInitialized,
    NotOwnedBySystemProgram,
    AddressDerivationFailed,
    AmountCannotBeZero,
    InsufficientUserBalance,
    OwnerNotTokenProgram,
    MintAccountMismatch,
    IncorrectTokenAccountOwner,
    InvalidMintAccount,
    InvalidMarketAccountOwner,
    MissingIndexHint,
    InvalidNonZeroInteger,
    InvalidInstructionData,
    IncorrectEventAuthority,
    EventAuthorityMustBeSigner,
    OrderWithPriceAlreadyExists,
    UserHasMaxOrders,
    OrderNotFound,
    ExponentUnderflow,
    ArithmeticOverflow,
    InvalidPriceMantissa,
    InvalidBiasedExponent,
    InfinityIsNotAFloat,
    PostOnlyWouldImmediatelyFill,
    AmountFilledVsTransferredMismatch,
}

impl From<DropsetError> for ProgramError {
    #[inline(always)]
    fn from(e: DropsetError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<OrderInfoError> for DropsetError {
    #[inline(always)]
    fn from(order_error: OrderInfoError) -> Self {
        match order_error {
            OrderInfoError::ExponentUnderflow => DropsetError::ExponentUnderflow,
            OrderInfoError::ArithmeticOverflow => DropsetError::ArithmeticOverflow,
            OrderInfoError::InvalidPriceMantissa => DropsetError::InvalidPriceMantissa,
            OrderInfoError::InvalidBiasedExponent => DropsetError::InvalidBiasedExponent,
            OrderInfoError::InfinityIsNotAFloat => DropsetError::InfinityIsNotAFloat,
            OrderInfoError::AmountCannotBeZero => DropsetError::AmountCannotBeZero,
        }
    }
}

impl From<DropsetError> for &'static str {
    fn from(value: DropsetError) -> Self {
        match value {
            DropsetError::InvalidInstructionTag => "Invalid instruction tag",
            DropsetError::InvalidInstructionEventTag => "Invalid instruction event tag",
            DropsetError::InsufficientByteLength => "Not enough bytes passed",
            DropsetError::InvalidSectorIndex => "Invalid sector index passed",
            DropsetError::NoFreeNodesLeft => "There are no free stack nodes left",
            DropsetError::InvalidAccountDiscriminant => "Invalid account discriminant",
            DropsetError::IndexOutOfBounds => "Index is out of bounds",
            DropsetError::InvalidIndexHint => "Index hint is invalid",
            DropsetError::UnalignedData => "Account data is unaligned",
            DropsetError::UnallocatedAccountData => "Account data hasn't been properly allocated",
            DropsetError::UserAlreadyExists => "User already has an existing seat",
            DropsetError::InvalidTokenProgram => "Invalid token program ID",
            DropsetError::AlreadyInitializedAccount => "Account has already been initialized",
            DropsetError::AccountNotInitialized => "Account hasn't been initialized",
            DropsetError::NotOwnedBySystemProgram => "Account is not owned by the system program",
            DropsetError::AddressDerivationFailed => "PDA derivation failed",
            DropsetError::AmountCannotBeZero => "Amount can't be zero",
            DropsetError::InsufficientUserBalance => "Insufficient user balance",
            DropsetError::OwnerNotTokenProgram => "Account owner must be a valid token program",
            DropsetError::MintAccountMismatch => "Mint account does not match",
            DropsetError::IncorrectTokenAccountOwner => "Incorrect associated token account owner",
            DropsetError::InvalidMintAccount => "Invalid mint account",
            DropsetError::InvalidMarketAccountOwner => "Invalid market account owner",
            DropsetError::MissingIndexHint => "Instruction data must include an index hint",
            DropsetError::InvalidNonZeroInteger => "Value passed must be greater than zero",
            DropsetError::InvalidInstructionData => "Instruction data is invalid",
            DropsetError::IncorrectEventAuthority => "The event authority passed isn't correct",
            DropsetError::EventAuthorityMustBeSigner => "The event authority isn't a signer",
            DropsetError::OrderWithPriceAlreadyExists => "An order with this price already exists",
            DropsetError::UserHasMaxOrders => "User already has the max number of open orders",
            DropsetError::OrderNotFound => "Order not found",
            DropsetError::ExponentUnderflow => "Order exponent underflow",
            DropsetError::ArithmeticOverflow => "Order arithmetic overflow",
            DropsetError::InvalidPriceMantissa => "Invalid price mantissa in price calculation",
            DropsetError::InvalidBiasedExponent => "Invalid biased exponent in price calculation",
            DropsetError::InfinityIsNotAFloat => "Can't convert infinity to a float value",
            DropsetError::PostOnlyWouldImmediatelyFill => "Post only order would immediately fill",
            DropsetError::AmountFilledVsTransferredMismatch => {
                "The amount filled doesn't match the amount transferred."
            }
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
