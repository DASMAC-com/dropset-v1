//! See [`MarketHeader`].

use solana_address::Address;
use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        sector::{
            LeSectorIndex,
            SectorIndex,
            LE_NIL,
        },
        transmutable::Transmutable,
        LeU32,
        LeU64,
        U32_SIZE,
        U64_SIZE,
    },
};

pub const MARKET_ACCOUNT_DISCRIMINANT: u64 = 0xd00d00b00b00f00du64;

/// The lightweight header for each market account. This header contains metadata used to interpret
/// a market's account data properly.
///
/// A market account’s data consists of a statically sized [`MarketHeader`] followed by its
/// dynamically sized `sectors` region stored as raw bytes: `&[u8]`.
///
/// The metadata stored in the market header is central to interpreting the structures contained
/// within the market’s `sectors` bytes. This region acts as an untagged union of data structures
/// that share a common iterable layout, where each item is a sector with some payload type `T`.
///
/// For example, [`MarketHeader::free_stack_top`] exposes the index of the top sector in the free
/// stack, allowing traversal of all available sectors. The payload type `T` in this case is
/// [`crate::state::free_stack::FreePayload`].
#[repr(C)]
#[derive(Clone, Debug)]
pub struct MarketHeader {
    /// The u64 market account's account discriminant as LE bytes.
    discriminant: LeU64,
    /// The u32 total number of fully initialized seats as LE bytes.
    num_seats: LeU32,
    /// The u32 total number of fully initialized bid orders as LE bytes.
    num_bids: LeU32,
    /// The u32 total number of fully initialized ask orders as LE bytes.
    num_asks: LeU32,
    /// The u32 total number of sectors in the free sectors stack as LE bytes.
    num_free_sectors: LeU32,
    /// The u32 sector index of the first sector in the free sectors stack as LE bytes.
    free_stack_top: LeSectorIndex,
    /// The u32 sector index of the first sector in the seat sectors DLL as LE bytes.
    seats_dll_head: LeSectorIndex,
    /// The u32 sector index of the last sector in the seat sectors DLL as LE bytes.
    seats_dll_tail: LeSectorIndex,
    /// The u32 sector index of the first sector in the bid sectors DLL as LE bytes.
    bids_dll_head: LeSectorIndex,
    /// The u32 sector index of the last sector in the bid sectors DLL as LE bytes.
    bids_dll_tail: LeSectorIndex,
    /// The u32 sector index of the first sector in the ask sectors DLL as LE bytes.
    asks_dll_head: LeSectorIndex,
    /// The u32 sector index of the last sector in the ask sectors DLL as LE bytes.
    asks_dll_tail: LeSectorIndex,
    /// The market's base mint public key.
    pub base_mint: Address,
    /// The market's quote mint public key.
    pub quote_mint: Address,
    /// The bump for the market PDA.
    pub market_bump: u8,
    /// The u64 number of events as LE bytes.
    num_events: LeU64,
    // Although not necessary, add extra padding to make this alignment 8.
    _padding: [u8; 3],
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for MarketHeader {
    #[allow(clippy::identity_op)]
    const LEN: usize = 0
    /* discriminant */     + size_of::<LeU64>()
    /* num_seats */        + size_of::<LeU32>()
    /* num_bids */         + size_of::<LeU32>()
    /* num_asks */         + size_of::<LeU32>()
    /* num_free_sectors */ + size_of::<LeU32>()
    /* free_stack_top */   + size_of::<LeSectorIndex>()
    /* seats_dll_head */   + size_of::<LeSectorIndex>()
    /* seats_dll_tail */   + size_of::<LeSectorIndex>()
    /* bids_dll_head */    + size_of::<LeSectorIndex>()
    /* bids_dll_tail */    + size_of::<LeSectorIndex>()
    /* asks_dll_head */    + size_of::<LeSectorIndex>()
    /* asks_dll_tail */    + size_of::<LeSectorIndex>()
    /* base_mint */        + size_of::<Address>()
    /* quote_mint */       + size_of::<Address>()
    /* market_bump */      + size_of::<u8>()
    /* num_events */       + size_of::<LeU64>()
    /* _padding */         + size_of::<[u8; 3]>();

    fn validate_bit_patterns(_bytes: &[u8]) -> DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(MarketHeader::LEN, size_of::<MarketHeader>());
const_assert_eq!(align_of::<MarketHeader>(), 1);

/// Helper macro to implement a getter + wrapping add/sub increment/decrement methods for a
/// `[u8; 4]` field. The field itself represents a u32 counter field for the number of elements
/// in a collection.
///
/// Once the program has sufficient test coverage, the wrapping add/sub operations could be
/// updated to cheaper, unchecked operations.
///
/// Generates:
/// - `fn $field(&self) -> u32`
/// - `fn increment_$field(&mut self)`
/// - `fn decrement_$field(&mut self)`
macro_rules! impl_u32_counter_field {
    ($field:ident) => {
        #[inline(always)]
        pub fn $field(&self) -> u32 {
            u32::from_le_bytes(self.$field)
        }

        paste::paste! {
            #[inline(always)]
            pub fn [<increment_ $field>](&mut self) {
                let $field = self.$field();
                // Debug assertion to catch bugs in development.
                // This is only a possible issue if the program logic itself is incorrect.
                debug_assert!($field < u32::MAX);
                self.$field = $field.wrapping_add(1).to_le_bytes();
            }

            #[inline(always)]
            pub fn [<decrement_ $field>](&mut self) {
                let $field = self.$field();
                // Debug assertion to catch bugs in development.
                // This is only a possible issue if the program logic itself is incorrect.
                debug_assert!($field > 0);
                self.$field = $field.wrapping_sub(1).to_le_bytes();
            }
        }
    };
}

/// Implements a getter and setter for a `[u8; 4]` field storing the little-endian bytes
/// representing a `SectorIndex`.
///
/// Generates:
/// - `fn $field(&self) -> SectorIndex`
/// - `fn set_$field(&mut self, index: SectorIndex)`
macro_rules! impl_get_set_sector_index_field {
    ($field:ident) => {
        #[inline(always)]
        pub fn $field(&self) -> SectorIndex {
            u32::from_le_bytes(self.$field)
        }

        paste::paste! {
            #[inline(always)]
            pub fn [<set_ $field>](&mut self, index: SectorIndex) {
                self.$field = index.to_le_bytes();
            }
        }
    };
}

impl MarketHeader {
    impl_u32_counter_field!(num_free_sectors);

    impl_u32_counter_field!(num_seats);

    impl_u32_counter_field!(num_bids);

    impl_u32_counter_field!(num_asks);

    impl_get_set_sector_index_field!(free_stack_top);

    impl_get_set_sector_index_field!(seats_dll_head);

    impl_get_set_sector_index_field!(seats_dll_tail);

    impl_get_set_sector_index_field!(bids_dll_head);

    impl_get_set_sector_index_field!(bids_dll_tail);

    impl_get_set_sector_index_field!(asks_dll_head);

    impl_get_set_sector_index_field!(asks_dll_tail);

    /// Initializes market header data to the header destination pointer with a `core::ptr::write`.
    ///
    /// # Safety
    ///
    /// Caller guarantees:
    /// - `header_dst_ptr` points to allocated memory with at least `MarketHeader::LEN` bytes.
    /// - The pointer has exclusive mutable access (no active borrows or aliases)
    #[inline(always)]
    pub unsafe fn init(
        header_dst_ptr: *mut MarketHeader,
        market_bump: u8,
        base_mint: &Address,
        quote_mint: &Address,
    ) {
        let header = MarketHeader {
            discriminant: MARKET_ACCOUNT_DISCRIMINANT.to_le_bytes(),
            num_seats: [0; U32_SIZE],
            num_bids: [0; U32_SIZE],
            num_asks: [0; U32_SIZE],
            num_free_sectors: [0; U32_SIZE],
            free_stack_top: LE_NIL,
            seats_dll_head: LE_NIL,
            seats_dll_tail: LE_NIL,
            bids_dll_head: LE_NIL,
            bids_dll_tail: LE_NIL,
            asks_dll_head: LE_NIL,
            asks_dll_tail: LE_NIL,
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            market_bump,
            num_events: [0; U64_SIZE],
            _padding: [0; 3],
        };
        core::ptr::write(header_dst_ptr, header);
    }

    #[inline(always)]
    pub fn verify_discriminant(&self) -> DropsetResult {
        if self.discriminant() != MARKET_ACCOUNT_DISCRIMINANT {
            return Err(DropsetError::InvalidAccountDiscriminant);
        }
        Ok(())
    }

    #[inline(always)]
    pub fn discriminant(&self) -> u64 {
        u64::from_le_bytes(self.discriminant)
    }

    #[inline(always)]
    pub fn num_events(&self) -> u64 {
        u64::from_le_bytes(self.num_events)
    }

    #[inline(always)]
    pub fn increment_num_events_by(&mut self, amount: u64) {
        self.num_events = (self.num_events().saturating_add(amount)).to_le_bytes();
    }
}
