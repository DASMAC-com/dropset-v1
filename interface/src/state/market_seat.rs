//! See [`MarketSeat`].

use solana_address::Address;
use static_assertions::const_assert_eq;

use crate::{
    error::{
        DropsetError,
        DropsetResult,
    },
    state::{
        sector::{
            AllBitPatternsValid,
            Payload,
            PAYLOAD_SIZE,
        },
        transmutable::Transmutable,
        user_order_sectors::UserOrderSectors,
        U64_SIZE,
    },
};

/// Represents a user's position within a market.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketSeat {
    /// The user's public key.
    pub user: Address,
    /// The u64 amount of base the maker can withdraw as LE bytes.
    base_available: [u8; U64_SIZE],
    /// The u64 amount of quote the maker can withdraw as LE bytes.
    quote_available: [u8; U64_SIZE],
    /// The mapping for a user's order prices to order sector indices.
    /// This facilitates O(1) indexing from a user's seat -> their orders.
    pub user_order_sectors: UserOrderSectors,
}

impl MarketSeat {
    pub fn new(user: Address, base: u64, quote: u64) -> Self {
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
    pub fn try_decrement_base_available(&mut self, amount: u64) -> DropsetResult {
        let remaining = price::checked_sub!(
            self.base_available(),
            amount,
            DropsetError::InsufficientUserBalance
        )?;
        self.set_base_available(remaining);

        Ok(())
    }

    #[inline(always)]
    pub fn try_decrement_quote_available(&mut self, amount: u64) -> DropsetResult {
        let remaining = price::checked_sub!(
            self.quote_available(),
            amount,
            DropsetError::InsufficientUserBalance
        )?;
        self.set_quote_available(remaining);

        Ok(())
    }

    #[inline(always)]
    pub fn try_increment_base_available(&mut self, amount: u64) -> DropsetResult {
        let new_amount = self.base_available().checked_add(amount).ok_or_else(|| {
            pinocchio::hint::cold_path();
            DropsetError::ArithmeticOverflow
        })?;
        self.set_base_available(new_amount);

        Ok(())
    }

    #[inline(always)]
    pub fn try_increment_quote_available(&mut self, amount: u64) -> DropsetResult {
        let new_amount = self.quote_available().checked_add(amount).ok_or_else(|| {
            pinocchio::hint::cold_path();
            DropsetError::ArithmeticOverflow
        })?;
        self.set_quote_available(new_amount);

        Ok(())
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
unsafe impl Transmutable for MarketSeat {
    const LEN: usize =
        size_of::<Address>() + size_of::<u64>() + size_of::<u64>() + UserOrderSectors::LEN;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(size_of::<MarketSeat>(), MarketSeat::LEN);
const_assert_eq!(align_of::<MarketSeat>(), 1);

// Safety: Const asserts ensure MarketSeat::LEN <= PAYLOAD_SIZE.
unsafe impl Payload for MarketSeat {}

const_assert_eq!(MarketSeat::LEN, PAYLOAD_SIZE);

// Safety: All bit patterns are valid.
unsafe impl AllBitPatternsValid for MarketSeat {}
