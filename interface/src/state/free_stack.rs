//! See [`Stack`].

use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        market_header::MarketHeader,
        sector::{
            AllBitPatternsValid,
            Payload,
            Sector,
            PAYLOAD_SIZE,
        },
        sector::{
            SectorIndex,
            NIL,
        },
        transmutable::Transmutable,
    },
};

/// Implements a stack allocator abstraction for managing freed sectors and reusing space
/// efficiently.
pub struct Stack<'a> {
    /// See [`MarketHeader`].
    header: &'a mut MarketHeader,
    /// The slab of bytes where all sector data exists, where each sector is an untagged union
    /// of (any possible sector type | FreePayload).
    sectors: &'a mut [u8],
}

#[repr(transparent)]
/// A free payload is the unused payload portion of the "free" variant of the untagged union of
/// each sector type (market seat, market order, etc).
/// Since a free sector only ever reads from the `next` field, it's not necessary to zero out the
/// payload bytes and thus they should be considered garbage data.
pub struct FreePayload(pub [u8; PAYLOAD_SIZE]);

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for FreePayload {
    const LEN: usize = PAYLOAD_SIZE;

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(FreePayload::LEN, size_of::<FreePayload>());
const_assert_eq!(1, align_of::<FreePayload>());

// Safety: FreePayload's size is checked below.
unsafe impl Payload for FreePayload {}

// Safety: All bit patterns are valid.
unsafe impl AllBitPatternsValid for FreePayload {}

const_assert_eq!(size_of::<FreePayload>(), PAYLOAD_SIZE);

impl<'a> Stack<'a> {
    pub fn new_from_parts(header: &'a mut MarketHeader, sectors: &'a mut [u8]) -> Self {
        Stack { header, sectors }
    }

    /// Push a sector at the sector index onto the stack as a free sector by zeroing out its data,
    /// setting its `next` to the current `top`, and updating the stack `top`.
    ///
    /// # Safety
    ///
    /// Caller guarantees `index` is in-bounds of the sector bytes.
    pub unsafe fn push_free_sector(&mut self, index: SectorIndex) {
        let curr_top = self.top();

        let sector = unsafe { Sector::from_sector_index_mut(self.sectors, index) };
        sector.zero_out_payload();

        sector.set_next(curr_top);
        self.set_top(index);
    }

    /// Initialize zeroed out bytes as free stack sectors.
    ///
    /// This should only be called directly after increasing the size of the account data, since the
    /// account data's bytes in that case are always zero-initialized.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - Account data from sector index `start` to `end` is already zeroed out bytes.
    /// - `start < end`
    /// - `end` is in-bounds of the account's data.
    /// - `start` and `end` are both non-NIL.
    pub unsafe fn convert_zeroed_bytes_to_free_sectors(
        &mut self,
        start: u32,
        end: u32,
    ) -> DropsetResult {
        // Debug check that the sector has been zeroed out.
        debug_assert!(
            start < end
                && (start..end).all(|i| {
                    // Safety: The safety contract guarantees the index is always in-bounds.
                    let sector = unsafe { Sector::from_sector_index_mut(self.sectors, i) };
                    sector.load_payload::<FreePayload>().0 == [0; PAYLOAD_SIZE]
                })
        );

        for index in (start..end).rev() {
            let curr_top = self.top();

            // Safety: The safety contract guarantees the index is always in-bounds.
            let sector = unsafe { Sector::from_sector_index_mut(self.sectors, index) };

            sector.set_next(curr_top);
            self.set_top(index);
            self.header.increment_num_free_sectors();
        }

        Ok(())
    }

    /// Tries to remove a free [`Sector`] and if successful, returns its [`SectorIndex`].
    ///
    /// An Ok([`SectorIndex`]) is always in-bounds and non-NIL.
    ///
    /// NOTE: If the returned index is discarded without being reinserted into a data structure
    /// (or pushed back onto the free stack), that sector becomes unreachable and is effectively
    /// leaked from future use.
    ///
    /// The sector's data is not zeroed prior to being removed, so when repurposing the freed
    /// sector for another data structure, the returned free sector's data should be considered
    /// invalid/garbage data until it's updated appropriately.
    pub fn pop_free_sector(&mut self) -> Result<SectorIndex, DropsetError> {
        // The free sector is the sector at the top of the stack.
        let free_index = self.top();

        if free_index == NIL {
            return Err(DropsetError::NoFreeSectorsRemaining);
        }

        Sector::check_in_bounds(self.sectors, free_index)?;
        // Safety: The free index was just checked as in-bounds.
        let sector_being_freed = unsafe { Sector::from_sector_index_mut(self.sectors, free_index) };

        // Copy the current top's `next` as that will become the new `top`.
        let new_top = sector_being_freed.next();

        self.set_top(new_top);
        self.header.decrement_num_free_sectors();

        // Now return the index of the freed sector.
        Ok(free_index)
    }

    #[inline(always)]
    pub fn top(&self) -> SectorIndex {
        self.header.free_stack_top()
    }

    #[inline(always)]
    pub fn set_top(&mut self, index: SectorIndex) {
        self.header.set_free_stack_top(index);
    }
}
