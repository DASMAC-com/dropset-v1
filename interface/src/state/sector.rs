//! Defines the size and sentinel constants for fixed-size storage sectors used to organize
//! and index market data efficiently in account memory.

use crate::state::U32_SIZE;

pub const SECTOR_SIZE: usize = 72;

/// A sentinel value that marks 1-past the last valid sector index.
///
/// This value will never appear naturally. Even at a sector size of 1 byte, Solana's max account
/// size of 10 MB would put the max sector index at ~10.5 mil — far less than u32::MAX.
pub const NIL: SectorIndex = u32::MAX;

// A sector index stored as little-endian bytes.
pub type LeSectorIndex = [u8; U32_SIZE];

/// A stride-based index into an array of sectors.
///
/// Index `i` maps to byte offset `i × SECTOR_SIZE` for a raw `sectors: &[u8]` slice.
pub type SectorIndex = u32;
