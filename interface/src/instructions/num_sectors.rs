use static_assertions::const_assert_eq;

use crate::{
    pack::{write_bytes, Pack},
    state::{transmutable::Transmutable, LeU16},
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct NumSectorsInstructionData {
    num_sectors: LeU16,
}

impl NumSectorsInstructionData {
    pub fn new(num_sectors: u16) -> Self {
        Self {
            num_sectors: num_sectors.to_le_bytes(),
        }
    }

    #[inline(always)]
    pub fn num_sectors(&self) -> u16 {
        u16::from_le_bytes(self.num_sectors)
    }
}

impl Pack<2> for NumSectorsInstructionData {
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 2]) {
        write_bytes(&mut dst[0..2], &self.num_sectors);
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for NumSectorsInstructionData {
    const LEN: usize = 2;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(
    NumSectorsInstructionData::LEN,
    size_of::<NumSectorsInstructionData>()
);
const_assert_eq!(1, align_of::<NumSectorsInstructionData>());
