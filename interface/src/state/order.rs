use price::{
    EncodedPrice,
    LeEncodedPrice,
    OrderInfo,
};
use static_assertions::const_assert_eq;

use crate::{
    error::DropsetResult,
    state::{
        linked_list::{
            LinkedListHeaderOperations,
            LinkedListIter,
        },
        market::Market,
        market_header::MarketHeader,
        market_seat::MarketSeat,
        sector::{
            AllBitPatternsValid,
            LeSectorIndex,
            Payload,
            SectorIndex,
            PAYLOAD_SIZE,
        },
        transmutable::Transmutable,
        user_order_sectors::{
            OrderSectors,
            UserOrderSectors,
        },
        U64_SIZE,
    },
};

/// A sector index returned by [OrdersCollection::find_new_order_next_index].
#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NextSectorIndex(pub(crate) SectorIndex);

impl From<NextSectorIndex> for SectorIndex {
    #[inline(always)]
    fn from(value: NextSectorIndex) -> Self {
        value.0
    }
}

/// Marker trait to indicate that a struct represents a collection of orders.
pub trait OrdersCollection: LinkedListHeaderOperations {
    /// The highest possible price in terms of price priority with respect to the collection type.
    /// This is not necessarily a valid price for an [Order]; it is intended for use in comparisons
    /// and sorting algorithms.
    const HIGHEST_PRIORITY_PRICE: EncodedPrice;

    /// Find the insertion point for a new order by returning what the new order sector's
    /// `next_index` should be after insertion.
    ///
    /// That is, given some `new` order, the list would be updated from this:
    ///
    /// `prev => next`
    /// To this:
    /// `prev => new => next`
    ///
    /// where this function returns the `next` sector's sector index.
    ///
    /// It also returns the current iterator position after searching, which can be used as a hint
    /// for subsequent searches. This is useful for batch operations since it facilitates inserting
    /// sorted orders in a single pass, as opposed to searching the whole list for each new order.
    fn find_new_order_next_index(
        list_iterator: LinkedListIter<'_>,
        new_order: &Order,
    ) -> (NextSectorIndex, SectorIndex);

    /// A post-only order must not execute immediately, so it must fail if it would cross the book
    /// and match against resting liquidity.
    fn post_only_crossing_check<H, S>(order: &Order, market: &Market<H, S>) -> DropsetResult
    where
        H: AsRef<MarketHeader>,
        S: AsRef<[u8]>;

    /// Returns the asset used as collateral when originally placing the order.
    fn get_order_collateral(order: &Order) -> u64;

    /// Tries to decrement the collateral available in a user's seat.
    fn try_decrement_seat_collateral_available(seat: &mut MarketSeat, amount: u64)
        -> DropsetResult;

    /// Tries to increment the collateral available in a user's seat.
    fn try_increment_seat_collateral_available(seat: &mut MarketSeat, amount: u64)
        -> DropsetResult;

    /// Returns a user's mapped order sectors corresponding to the collection type.
    fn get_order_sectors(user_order_sectors: &UserOrderSectors) -> &OrderSectors;

    /// Returns a user's mapped order sectors corresponding to the collection type.
    fn get_mut_order_sectors(user_order_sectors: &mut UserOrderSectors) -> &mut OrderSectors;

    /// Returns whether or not the first price has a higher priority than the second with respect to
    /// the collection type.
    fn has_higher_price_priority(a: &EncodedPrice, b: &EncodedPrice) -> bool;
}

const ORDER_PADDING: usize =
    PAYLOAD_SIZE - (size_of::<LeEncodedPrice>() + size_of::<LeSectorIndex>() + U64_SIZE + U64_SIZE);

/// Represents a maker order in the orderbook.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Order {
    /// The LE bytes representing an [`EncodedPrice`].
    encoded_price: LeEncodedPrice,
    /// This enables O(1) indexing from a user/maker's orders -> their seat.
    user_seat_index: LeSectorIndex,
    /// The u64 number of base atoms left remaining to fill as LE bytes.
    base_remaining: [u8; U64_SIZE],
    /// The u64 number of quote atoms left remaining to fill as LE bytes.
    quote_remaining: [u8; U64_SIZE],
    /// Padding to fill the rest of the sector payload size.
    _padding: [u8; ORDER_PADDING],
}

impl Order {
    /// Create a new order from the order info and the user seat.
    #[inline(always)]
    pub fn new(order_info: OrderInfo, user_seat_index: SectorIndex) -> Self {
        Self {
            encoded_price: order_info.encoded_price.into(),
            user_seat_index: user_seat_index.to_le_bytes(),
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
    pub fn encoded_price(&self) -> EncodedPrice {
        let as_u32 = u32::from_le_bytes(self.encoded_price.as_array());

        // Safety: `self.encoded_price` is always a valid encoded price and `EncodedPrice` is
        // repr(transparent) over a u32.
        unsafe { core::mem::transmute::<u32, EncodedPrice>(as_u32) }
    }

    #[inline(always)]
    pub fn user_seat(&self) -> u32 {
        u32::from_le_bytes(self.user_seat_index)
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

    #[inline(always)]
    pub fn collateral_amount<T: OrdersCollection>(&self) -> u64 {
        T::get_order_collateral(self)
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
    const LEN: usize = size_of::<Order>();

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid: no enums, bools, or other types with invalid states.
        Ok(())
    }
}

const_assert_eq!(size_of::<Order>(), PAYLOAD_SIZE);
const_assert_eq!(align_of::<Order>(), 1);

// Safety: Const asserts ensure size_of::<Order>() == PAYLOAD_SIZE.
unsafe impl Payload for Order {}

// Safety: All bit patterns are valid.
unsafe impl AllBitPatternsValid for Order {}

#[cfg(test)]
mod tests {
    use price::{
        biased_exponent,
        to_order_info,
        EncodedPrice,
        OrderInfoArgs,
    };

    use super::*;

    #[test]
    fn new_order_happy_path() {
        let order_info =
            to_order_info((10_000_000, 5, 8, 0).into()).expect("Should create order info");
        let base_in_order = order_info.base_atoms;
        let quote_in_order = order_info.quote_atoms;
        let encoded_price_in_order = order_info.encoded_price;
        let user_seat = 17;
        let order = Order::new(order_info, user_seat);
        assert_eq!(base_in_order, order.base_remaining());
        assert_eq!(quote_in_order, order.quote_remaining());
        assert_eq!(encoded_price_in_order, order.encoded_price());
        assert_eq!(user_seat, order.user_seat());
    }

    #[test]
    fn order_mutators() {
        let order_info = to_order_info(OrderInfoArgs::new(
            10_000_000,
            5,
            biased_exponent!(7),
            biased_exponent!(0),
        ))
        .expect("Should create order info");
        let user_seat = 17;
        let mut order = Order::new(order_info.clone(), user_seat);
        assert_eq!(order.base_remaining(), 50_000_000);
        assert_eq!(order.quote_remaining(), 50_000_000);
        let base_after = 111_111_111;
        let quote_after: u64 = 222_222_222;
        order.set_base_remaining(base_after);
        order.set_quote_remaining(quote_after);
        assert_eq!(order.base_remaining(), base_after);
        assert_eq!(order.quote_remaining(), quote_after);
    }

    #[test]
    fn test_as_bytes() {
        const BASE_ATOMS: u64 = 1234;
        const QUOTE_ATOMS: u64 = 4321;
        let order_info = OrderInfo {
            encoded_price: EncodedPrice::zero(),
            base_atoms: BASE_ATOMS,
            quote_atoms: QUOTE_ATOMS,
        };
        const USER_SEAT: SectorIndex = 9191;
        let order = Order::new(order_info, USER_SEAT);
        assert_eq!(
            [
                &0u32.to_le_bytes(),                // Encoded price.
                &USER_SEAT.to_le_bytes(),           // User seat.
                BASE_ATOMS.to_le_bytes().as_ref(),  // Base remaining.
                QUOTE_ATOMS.to_le_bytes().as_ref(), // Quote remaining.
                [0u8; ORDER_PADDING].as_ref(),      // Padding.
            ]
            .concat(),
            order.as_bytes()
        );
    }
}
