pub mod free_stack;
pub mod linked_list;
pub mod market;
pub mod market_header;
pub mod market_seat;
pub mod node;
pub mod sector;
pub mod transmutable;

pub const U16_SIZE: usize = core::mem::size_of::<u16>();
pub const U32_SIZE: usize = core::mem::size_of::<u32>();
pub const U64_SIZE: usize = core::mem::size_of::<u64>();

/// Alias type for a u16 stored as little-endian bytes.
pub type LeU16 = [u8; U16_SIZE];
/// Alias type for a u32 stored as little-endian bytes.
pub type LeU32 = [u8; U32_SIZE];
/// Alias type for a u64 stored as little-endian bytes.
pub type LeU64 = [u8; U64_SIZE];

pub const SYSTEM_PROGRAM_ID: pinocchio::pubkey::Pubkey =
    pinocchio_pubkey::pubkey!("11111111111111111111111111111111");
