pub mod sector;

pub const U16_SIZE: usize = core::mem::size_of::<u16>();
pub const U32_SIZE: usize = core::mem::size_of::<u32>();
pub const U64_SIZE: usize = core::mem::size_of::<u64>();

pub const SYSTEM_PROGRAM_ID: pinocchio::pubkey::Pubkey =
    pinocchio_pubkey::pubkey!("11111111111111111111111111111111");
