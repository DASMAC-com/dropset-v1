use static_assertions::const_assert_eq;

use crate::{
    error::DropsetError,
    pack::{write_bytes, Pack},
    state::{sector::NonNilSectorIndex, transmutable::Transmutable, U32_SIZE},
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct CloseInstructionData {
    /// A hint as to which sector index the calling user is located in the sectors array.
    sector_index_hint: [u8; U32_SIZE],
}

impl CloseInstructionData {
    pub fn new(sector_index_hint: NonNilSectorIndex) -> Self {
        CloseInstructionData {
            sector_index_hint: sector_index_hint.into(),
        }
    }

    #[inline(always)]
    pub fn try_sector_index_hint(&self) -> Result<NonNilSectorIndex, DropsetError> {
        NonNilSectorIndex::new(self.sector_index_hint.into())
    }
}

impl Pack<8> for CloseInstructionData {
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 8]) {
        write_bytes(&mut dst[0..8], &self.sector_index_hint);
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for CloseInstructionData {
    const LEN: usize = 4;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(CloseInstructionData::LEN, size_of::<CloseInstructionData>());
const_assert_eq!(1, align_of::<CloseInstructionData>());
