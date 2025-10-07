use crate::state::U32_SIZE;

pub const SECTOR_SIZE: usize = 72;

/// A sentinel sector index.
///
/// u32::MAX is safe as a sentinel value because the max sector index in an account is essentially
/// MAX_SOLANA_ACCOUNT_SIZE / SECTOR_SIZE. Even at a sector size of 1 byte, the max account size
/// of 10 megabytes would mean the max sector index (~10.5 million) is still far less than u32::MAX.
pub const NIL: SectorIndex = SectorIndex(u32::MAX);

// An alias type for readability.
pub type LeSectorIndex = [u8; U32_SIZE];

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A stride-based index into an array of sectors.
///
/// Index `i` maps to byte offset `i × SECTOR_SIZE` for a raw `sectors: &[u8]` slice.
pub struct SectorIndex(pub u32);

impl SectorIndex {
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.0 == NIL.0
    }
}

impl From<[u8; U32_SIZE]> for SectorIndex {
    fn from(value: [u8; U32_SIZE]) -> Self {
        SectorIndex(u32::from_le_bytes(value))
    }
}

impl From<SectorIndex> for [u8; U32_SIZE] {
    #[inline(always)]
    fn from(value: SectorIndex) -> Self {
        value.0.to_le_bytes()
    }
}
