use static_assertions::const_assert_eq;

use crate::{
    pack::{write_bytes, Pack},
    state::{
        sector::{LeSectorIndex, SectorIndex, NIL},
        transmutable::Transmutable,
        LeU64,
    },
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct AmountInstructionData {
    /// The amount to deposit or withdraw.
    amount: LeU64,
    /// A hint as to which sector index the calling user is located in the sectors array.
    /// If `NIL`, it's treated as no hint. This avoids the need for a custom COption type.
    sector_index_hint: LeSectorIndex,
}

impl AmountInstructionData {
    pub fn new(amount: u64, sector_index_hint: Option<SectorIndex>) -> Self {
        AmountInstructionData {
            amount: amount.to_le_bytes(),
            sector_index_hint: sector_index_hint.unwrap_or(NIL).into(),
        }
    }

    #[inline(always)]
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    #[inline(always)]
    pub fn sector_index_hint(&self) -> Option<SectorIndex> {
        let index: SectorIndex = self.sector_index_hint.into();
        let not_nil = !index.is_nil();

        // If the sector index hint bytes are equal to `NIL`, return None, otherwise Some(index).
        not_nil.then_some(index)
    }
}

impl Pack<12> for AmountInstructionData {
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 12]) {
        write_bytes(&mut dst[0..8], &self.amount);
        write_bytes(&mut dst[8..12], &self.sector_index_hint);
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for AmountInstructionData {
    const LEN: usize = 12;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(
    AmountInstructionData::LEN,
    size_of::<AmountInstructionData>()
);
const_assert_eq!(1, align_of::<AmountInstructionData>());
