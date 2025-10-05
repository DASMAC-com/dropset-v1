use crate::{error::DropsetError, state::U32_SIZE};

pub const SECTOR_SIZE: usize = 72;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// A stride-based index into an array of sectors.
///
/// Index `i` maps to byte offset `i × SECTOR_SIZE` for a raw `sectors: &[u8]` slice.
pub struct SectorIndex(pub u32);

#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NonNilSectorIndex(SectorIndex);

impl NonNilSectorIndex {
    /// Checks that the index is not NIL.
    pub fn new(index: SectorIndex) -> Result<Self, DropsetError> {
        if index.is_nil() {
            return Err(DropsetError::InvalidSectorIndex);
        }
        Ok(Self(index))
    }

    #[inline(always)]
    pub fn get(&self) -> SectorIndex {
        self.0
    }

    /// Caller should ensure that the index passed to this function is non-NIL.
    pub fn new_unchecked(index: SectorIndex) -> Self {
        debug_assert_ne!(index, NIL);
        Self(index)
    }
}

pub const NIL: SectorIndex = SectorIndex(u32::MAX);
pub const NIL_LE: LeSectorIndex = LeSectorIndex(NIL.0.to_le_bytes());

impl From<SectorIndex> for u32 {
    #[inline(always)]
    fn from(value: SectorIndex) -> Self {
        value.0
    }
}

impl SectorIndex {
    #[inline(always)]
    pub fn is_nil(&self) -> bool {
        self.0 == NIL.0
    }
}

/// A nominal helper newtype to avoid get/set boilerplate and copy/paste errors on the little endian
/// representation of a nominal `SectorIndex` type.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LeSectorIndex(pub [u8; U32_SIZE]);

impl LeSectorIndex {
    #[inline(always)]
    /// A helper function to convert an `LeSectorIndex`'s inner bytes to a `SectorIndex`.
    pub fn get(&self) -> SectorIndex {
        SectorIndex(u32::from_le_bytes(self.0))
    }

    #[inline(always)]
    /// A helper function to set the inner bytes of an `LeSectorIndex` to the bytes of a passed
    /// `SectorIndex`.
    pub fn set(&mut self, index: SectorIndex) {
        self.0 = index.0.to_le_bytes();
    }
}
