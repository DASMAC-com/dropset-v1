use crate::{
    pack::{write_bytes, Pack},
    state::{
        sector::{LeSectorIndex, NonNilSectorIndex, SectorIndex},
        transmutable::Transmutable,
        U64_SIZE,
    },
};
use core::mem::MaybeUninit;

#[repr(C)]
pub struct AmountInstructionData {
    /// The amount to deposit or withdraw.
    amount: [u8; U64_SIZE],
    /// A hint as to which sector index the calling user is located in the sectors array.
    /// The getter for this field exposes it as an Option<NonNilSectorIndex>, where `NIL` as the
    /// hint is equivalent to None.
    sector_index_hint: LeSectorIndex,
}

impl AmountInstructionData {
    /// NIL as the sector index hint is the semantic equivalent of None here.
    pub fn new(amount: u64, sector_index_hint: SectorIndex) -> Self {
        AmountInstructionData {
            amount: amount.to_le_bytes(),
            sector_index_hint: LeSectorIndex(sector_index_hint.0.to_le_bytes()),
        }
    }

    #[inline(always)]
    pub fn amount(&self) -> u64 {
        u64::from_le_bytes(self.amount)
    }

    #[inline(always)]
    pub fn sector_index_hint(&self) -> Option<NonNilSectorIndex> {
        let hint = self.sector_index_hint.get();
        hint.is_nil()
            .then_some(NonNilSectorIndex::new_unchecked(hint))
    }
}

impl Pack<8> for AmountInstructionData {
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; 8]) {
        write_bytes(dst, &self.amount);
    }
}

unsafe impl Transmutable for AmountInstructionData {
    const LEN: usize = 8;
}
