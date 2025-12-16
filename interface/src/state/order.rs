use price::{
    EncodedPrice,
    LeEncodedPrice,
};
use static_assertions::const_assert_eq;

use crate::state::{
    node::{
        NodePayload,
        NODE_PAYLOAD_SIZE,
    },
    sector::{
        LeSectorIndex,
        SectorIndex,
    },
    transmutable::Transmutable,
    U64_SIZE,
};

const ORDER_PADDING: usize = NODE_PAYLOAD_SIZE
    - (size_of::<LeEncodedPrice>() + size_of::<LeSectorIndex>() + U64_SIZE + U64_SIZE);

/// Represents a maker order in the orderbook.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The LE bytes representing an [`EncodedPrice`].
    encoded_price: LeEncodedPrice,
    /// This enables O(1) indexing from a user/maker's orders -> their seat.
    user_seat: LeSectorIndex,
    /// The u64 number of atoms left remaining to fill as LE bytes.
    remaining: [u8; U64_SIZE],
    /// The u64 initial order size (in atoms) when posted as LE bytes.
    initial: [u8; U64_SIZE],
    /// Padding to fill the rest of the node payload size.
    _padding: [u8; ORDER_PADDING],
}

impl Order {
    #[inline(always)]
    pub fn new(encoded_price: EncodedPrice, user_seat: SectorIndex, order_size: u64) -> Self {
        let le_size = order_size.to_le_bytes();
        Self {
            encoded_price: encoded_price.into(),
            user_seat: user_seat.to_le_bytes(),
            remaining: le_size,
            initial: le_size,
            _padding: [0u8; ORDER_PADDING],
        }
    }

    #[inline(always)]
    pub fn le_encoded_price(&self) -> &LeEncodedPrice {
        &self.encoded_price
    }

    #[inline(always)]
    pub fn encoded_price(&self) -> u32 {
        u32::from_le_bytes(self.encoded_price.as_array())
    }

    #[inline(always)]
    pub fn user_seat(&self) -> u32 {
        u32::from_le_bytes(self.user_seat)
    }

    #[inline(always)]
    pub fn remaining(&self) -> u64 {
        u64::from_le_bytes(self.remaining)
    }

    #[inline(always)]
    pub fn set_remaining(&mut self, amount: u64) {
        self.remaining = amount.to_le_bytes();
    }

    #[inline(always)]
    pub fn initial(&self) -> u64 {
        u64::from_le_bytes(self.initial)
    }

    #[inline(always)]
    pub fn set_initial(&mut self, amount: u64) {
        self.initial = amount.to_le_bytes();
    }

    /// This method is sound because:
    ///
    /// - `Self` is exactly `Self::LEN` bytes.
    /// - Size and alignment are verified with const assertions.
    /// - All fields are byte-safe, `Copy`, non-pointer/reference u8 arrays.
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8; Self::LEN] {
        unsafe { &*(self as *const Self as *const [u8; Self::LEN]) }
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for Order {
    const LEN: usize = NODE_PAYLOAD_SIZE;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(size_of::<Order>(), NODE_PAYLOAD_SIZE);
const_assert_eq!(align_of::<Order>(), 1);

// Safety: Const asserts ensure size_of::<Order>() == NODE_PAYLOAD_SIZE.
unsafe impl NodePayload for Order {}
