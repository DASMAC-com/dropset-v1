use static_assertions::const_assert_eq;

use crate::{
    pack::{write_bytes, Pack},
    state::{
        sector::{LeSectorIndex, NonNilSectorIndex, SectorIndex},
        transmutable::Transmutable,
    },
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct CloseInstructionData {
    /// A hint as to which sector index the calling user is located in the sectors array.
    /// The getter for this field exposes it as an Option<NonNilSectorIndex>, where `NIL` as the
    /// hint is equivalent to None.
    sector_index_hint: LeSectorIndex,
}

impl CloseInstructionData {
    /// NIL as the sector index hint is the semantic equivalent of None here.
    pub fn new(sector_index_hint: SectorIndex) -> Self {
        CloseInstructionData {
            sector_index_hint: LeSectorIndex(sector_index_hint.0.to_le_bytes()),
        }
    }

    #[inline(always)]
    pub fn sector_index_hint(&self) -> Option<NonNilSectorIndex> {
        let hint = self.sector_index_hint.get();
        hint.is_nil()
            .then_some(NonNilSectorIndex::new_unchecked(hint))
    }
}

impl Pack<8> for CloseInstructionData {
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 8]) {
        write_bytes(dst, &self.sector_index_hint.0);
    }
}

unsafe impl Transmutable for CloseInstructionData {
    const LEN: usize = 4;
}

const_assert_eq!(CloseInstructionData::LEN, size_of::<CloseInstructionData>());
