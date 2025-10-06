use static_assertions::const_assert_eq;

use crate::{
    error::DropsetError,
    pack::{write_bytes, Pack},
    state::{
        sector::{LeSectorIndex, NonNilSectorIndex},
        transmutable::Transmutable,
    },
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct CloseInstructionData {
    /// A hint as to which sector index the calling user is located in the sectors array.
    sector_index_hint: LeSectorIndex,
}

impl CloseInstructionData {
    pub fn new(sector_index_hint: NonNilSectorIndex) -> Self {
        CloseInstructionData {
            sector_index_hint: LeSectorIndex(sector_index_hint.get().0.to_le_bytes()),
        }
    }

    #[inline(always)]
    pub fn try_sector_index_hint(&self) -> Result<NonNilSectorIndex, DropsetError> {
        let hint = self.sector_index_hint.get();
        NonNilSectorIndex::new(hint)
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
