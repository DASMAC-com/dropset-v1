//! See [`MarketSeat`].

use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::state::{
    node::{
        NodePayload,
        NODE_PAYLOAD_SIZE,
    },
    transmutable::Transmutable,
    user_order_sectors::UserOrderSectors,
    U64_SIZE,
};

/// Represents a user's position within a market.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSeat {
    /// The user's public key.
    pub user: Pubkey,
    /// The u64 amount of base the maker can withdraw as LE bytes.
    /// Updated on place, cancel, deposit, withdraw.
    base_available: [u8; U64_SIZE],
    /// The u64 amount of quote the maker can withdraw as LE bytes.
    /// Updated on place, cancel, deposit, withdraw.
    quote_available: [u8; U64_SIZE],
    /// The mapping for a user's order prices to order sector indices.
    /// This facilitates O(1) indexing from a user's seat -> their orders.
    user_order_sectors: UserOrderSectors,
}

impl MarketSeat {
    pub fn new(user: Pubkey, base: u64, quote: u64) -> Self {
        MarketSeat {
            user,
            base_available: base.to_le_bytes(),
            quote_available: quote.to_le_bytes(),
            user_order_sectors: UserOrderSectors::default(),
        }
    }

    #[inline(always)]
    pub fn base_available(&self) -> u64 {
        u64::from_le_bytes(self.base_available)
    }

    #[inline(always)]
    pub fn set_base_available(&mut self, amount: u64) {
        self.base_available = amount.to_le_bytes();
    }

    #[inline(always)]
    pub fn quote_available(&self) -> u64 {
        u64::from_le_bytes(self.quote_available)
    }

    #[inline(always)]
    pub fn set_quote_available(&mut self, amount: u64) {
        self.quote_available = amount.to_le_bytes();
    }

    #[inline(always)]
    pub fn as_array(&self) -> &[u8; MarketSeat::LEN] {
        // Safety:
        // - `MarketSeat` is always `LEN` bytes; size and alignment are checked with const asserts.
        // - All fields are byte-safe, `Copy`, non-pointer/reference u8 arrays.
        unsafe { &*(self as *const Self as *const [u8; MarketSeat::LEN]) }
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for MarketSeat {
    const LEN: usize = NODE_PAYLOAD_SIZE;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(size_of::<MarketSeat>(), NODE_PAYLOAD_SIZE);
const_assert_eq!(align_of::<MarketSeat>(), 1);

// Safety: Const asserts ensure size_of::<MarketSeat>() == NODE_PAYLOAD_SIZE.
unsafe impl NodePayload for MarketSeat {}
