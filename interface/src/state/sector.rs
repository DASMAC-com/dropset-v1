use crate::{error::DropsetError, state::U32_SIZE};

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

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NonNilSectorIndex(pub SectorIndex);

impl NonNilSectorIndex {
    /// Checks that the index is not NIL.
    pub fn new(index: SectorIndex) -> Result<Self, DropsetError> {
        if index.is_nil() {
            return Err(DropsetError::InvalidSectorIndex);
        }
        Ok(Self(index))
    }

    /// Caller should ensure that the index passed to this function is non-NIL.
    ///
    /// # Safety
    ///
    /// This method does not immediately cause UB but can cause UB in other methods that operate
    /// under a non-nil index invariant being passed to it.
    ///
    /// Caller guarantees that the index passed to this method *is definitively* non-NIL.
    pub unsafe fn new_unchecked(index: SectorIndex) -> Self {
        debug_assert_ne!(index, NIL);
        Self(index)
    }
}

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

impl From<[u8; U32_SIZE]> for NonNilSectorIndex {
    fn from(value: [u8; U32_SIZE]) -> Self {
        NonNilSectorIndex(SectorIndex::from(value))
    }
}

impl From<SectorIndex> for [u8; U32_SIZE] {
    #[inline(always)]
    fn from(value: SectorIndex) -> Self {
        value.0.to_le_bytes()
    }
}

impl From<NonNilSectorIndex> for [u8; U32_SIZE] {
    #[inline(always)]
    fn from(value: NonNilSectorIndex) -> Self {
        value.0.into()
    }
}
