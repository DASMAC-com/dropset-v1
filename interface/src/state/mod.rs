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

pub const SYSTEM_PROGRAM_ID: pinocchio::pubkey::Pubkey =
    pinocchio_pubkey::pubkey!("11111111111111111111111111111111");
