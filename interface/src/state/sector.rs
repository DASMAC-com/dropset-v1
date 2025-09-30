use crate::state::U32_SIZE;

pub const SECTOR_SIZE: usize = 56;

#[repr(transparent)]
#[derive(Eq, PartialEq)]
/// The physical sector index of some slab of bytes. Sectors correspond directly to the byte offset
/// as a factor of the sector type's size.
pub struct SectorIndex(pub u32);

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
    pub fn get(&self) -> SectorIndex {
        SectorIndex(u32::from_le_bytes(self.0))
    }

    #[inline(always)]
    pub fn set(&mut self, index: SectorIndex) {
        self.0 = index.0.to_le_bytes();
    }
}
