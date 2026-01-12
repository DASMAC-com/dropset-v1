use price::{
    LeEncodedPrice,
    OrderInfo,
};
use static_assertions::const_assert_eq;

use crate::{
    error::DropsetResult,
    state::{
        linked_list::{
            LinkedList,
            LinkedListOperations,
        },
        market::Market,
        market_header::MarketHeader,
        node::{
            AllBitPatternsValid,
            NodePayload,
            NODE_PAYLOAD_SIZE,
        },
        sector::{
            LeSectorIndex,
            SectorIndex,
        },
        transmutable::Transmutable,
        U64_SIZE,
    },
};

/// Marker trait to indicate that a struct represents a collection of orders.
pub trait OrdersCollection {
    /// Find the insertion point for a new order by returning what the new order node's `next_index`
    /// should be after insertion.
    ///
    /// That is, given some `new` order, the list would be updated from this:
    ///
    /// `prev => next`
    /// To this:
    /// `prev => new => next`
    ///
    /// where this function returns the `next` node's sector index.
    fn find_new_order_next_index<T: OrdersCollection + LinkedListOperations>(
        list: &LinkedList<'_, T>,
        new_order: &Order,
    ) -> SectorIndex;

    /// A post-only order must not execute immediately, so it must fail if it would cross the book
    /// and match against resting liquidity.
    fn post_only_crossing_check<H, S>(order: &Order, market: &Market<H, S>) -> DropsetResult
    where
        H: AsRef<MarketHeader>,
        S: AsRef<[u8]>;
}

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
    /// The u64 number of base atoms left remaining to fill as LE bytes.
    base_remaining: [u8; U64_SIZE],
    /// The u64 number of quote atoms left remaining to fill as LE bytes.
    quote_remaining: [u8; U64_SIZE],
    /// Padding to fill the rest of the node payload size.
    _padding: [u8; ORDER_PADDING],
}

impl Order {
    /// Create a new order from the order info and the user seat.
    #[inline(always)]
    pub fn new(order_info: OrderInfo, user_seat: SectorIndex) -> Self {
        Self {
            encoded_price: order_info.encoded_price.into(),
            user_seat: user_seat.to_le_bytes(),
            base_remaining: order_info.base_atoms.to_le_bytes(),
            quote_remaining: order_info.quote_atoms.to_le_bytes(),
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
    pub fn base_remaining(&self) -> u64 {
        u64::from_le_bytes(self.base_remaining)
    }

    #[inline(always)]
    pub fn set_base_remaining(&mut self, amount: u64) {
        self.base_remaining = amount.to_le_bytes();
    }

    #[inline(always)]
    pub fn quote_remaining(&self) -> u64 {
        u64::from_le_bytes(self.quote_remaining)
    }

    #[inline(always)]
    pub fn set_quote_remaining(&mut self, amount: u64) {
        self.quote_remaining = amount.to_le_bytes();
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

// Safety: All bit patterns are valid.
unsafe impl AllBitPatternsValid for Order {}
