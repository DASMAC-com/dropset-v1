use pinocchio::pubkey::Pubkey;
use static_assertions::const_assert_eq;

use crate::state::{
    node::{
        NodePayload,
        NODE_PAYLOAD_SIZE,
    },
    transmutable::Transmutable,
    LeU64,
};

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSeat {
    /// The user's public key.
    pub user: Pubkey,
    /// Amount of base token deposited.
    base_deposited: LeU64,
    /// Amount of quote token deposited.
    quote_deposited: LeU64,
    /// Amount of base token available.
    base_available: LeU64,
    /// Amount of quote token available.
    quote_available: LeU64,
}

impl MarketSeat {
    pub fn new(user: Pubkey, base: u64, quote: u64) -> Self {
        MarketSeat {
            user,
            base_deposited: base.to_le_bytes(),
            quote_deposited: quote.to_le_bytes(),
            base_available: base.to_le_bytes(),
            quote_available: quote.to_le_bytes(),
        }
    }

    #[inline(always)]
    pub fn base_deposited(&self) -> u64 {
        u64::from_le_bytes(self.base_deposited)
    }

    #[inline(always)]
    pub fn set_base_deposited(&mut self, amount: u64) {
        self.base_deposited = amount.to_le_bytes();
    }

    #[inline(always)]
    pub fn quote_deposited(&self) -> u64 {
        u64::from_le_bytes(self.quote_deposited)
    }

    #[inline(always)]
    pub fn set_quote_deposited(&mut self, amount: u64) {
        self.quote_deposited = amount.to_le_bytes();
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
