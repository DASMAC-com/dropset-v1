use price::{
    EncodedPrice,
    LeEncodedPrice,
};
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
    },
};

/// The max number of orders a single user/address can have for a single market for bids or asks.
/// That is, each user can have [`MAX_ORDERS`] bids and [`MAX_ORDERS`] asks for a single market.
pub const MAX_ORDERS: u8 = 5;

/// The [`OrderSectors`] that maps the prices of a user's bids and asks to their corresponding
/// orders' sector indices in the market account data.
///
/// `bids` and `asks` both have a maximum [`MAX_ORDERS`] orders.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserOrderSectors {
    pub bids: OrderSectors,
    pub asks: OrderSectors,
}

/// An array of [`MAX_ORDERS`] [`PriceToIndex`]s that maps unique prices to a sector index.
///
/// By default, each [`PriceToIndex`] represents an unused item by mapping an encoded price u32
/// value of `0` to the [`LE_NIL`] sector index.
#[repr(transparent)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderSectors([PriceToIndex; MAX_ORDERS as usize]);

impl Default for OrderSectors {
    fn default() -> Self {
        Self([PriceToIndex::new_free(); MAX_ORDERS as usize])
    }
}

impl OrderSectors {
    /// Attempt to find and return the sector index for the order corresponding to the passed
    /// encoded price.
    #[inline(always)]
    pub fn get(&self, target_price: &LeEncodedPrice) -> Option<SectorIndex> {
        self.0.iter().find_map(
            |PriceToIndex {
                 encoded_price,
                 sector_index,
             }| {
                match encoded_price.as_slice() == target_price.as_slice() {
                    true => Some(u32::from_le_bytes(*sector_index)),
                    false => None,
                }
            },
        )
    }

    /// Fallibly add a [`PriceToIndex`] to a user's orders.
    ///
    /// Fails if the user already has [`MAX_ORDERS`] or the price already has an existing order.
    ///
    /// The `sector_index` passed to this method should be non-NIL or the node after mutation will
    /// continue to be treated as a free node.
    #[inline(always)]
    pub fn add(
        &mut self,
        new_price: &LeEncodedPrice,
        order_index: &LeSectorIndex,
    ) -> DropsetResult {
        // Check if the price already exists in a node and fail early if it does.
        if self
            .iter()
            .any(|node| node.encoded_price.as_slice() == new_price.as_slice())
        {
            return Err(DropsetError::OrderWithPriceAlreadyExists);
        }

        let node = self
            .iter_mut()
            .find(|node| node.is_free())
            .ok_or(DropsetError::UserHasMaxOrders)?;

        node.encoded_price = *new_price;
        node.sector_index = *order_index;

        Ok(())
    }

    /// Fallibly remove a [`PriceToIndex`] from a user's orders.
    ///
    /// Fails if the user does not have an order corresponding to the passed encoded price.
    ///
    /// Note that the encoded price does not have to be validated since it's doing a simple match
    /// on equality and isn't stored anywhere.
    ///
    /// Returns the mapped order's sector index.
    #[inline(always)]
    pub fn remove(&mut self, encoded_price: u32) -> Result<LeSectorIndex, DropsetError> {
        let node = self
            .0
            .iter_mut()
            .find(|node| node.encoded_price.as_slice() == &encoded_price.to_le_bytes())
            .ok_or(DropsetError::OrderNotFound)?;

        let sector_index = node.sector_index;

        node.encoded_price = LeEncodedPrice::zero();
        node.sector_index = LE_NIL;

        Ok(sector_index)
    }

    #[inline(always)]
    pub fn iter(&self) -> core::slice::Iter<'_, PriceToIndex> {
        self.0.iter()
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, PriceToIndex> {
        self.0.iter_mut()
    }
}

/// The paired encoded price and sector index for an order.
///
/// If the sector index equals [`LE_NIL`], it's considered a freed node, otherwise, it contains an
/// existing, valid pair of encoded price to sector index.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PriceToIndex {
    pub encoded_price: LeEncodedPrice,
    pub sector_index: LeSectorIndex,
}

impl PriceToIndex {
    /// Create a new free node.
    #[inline(always)]
    pub fn new_free() -> Self {
        Self {
            encoded_price: LeEncodedPrice::zero(),
            sector_index: LE_NIL,
        }
    }

    /// Create a new encoded price to sector index node.
    #[inline(always)]
    pub fn new(encoded_price: EncodedPrice, sector_index: &SectorIndex) -> Self {
        Self {
            encoded_price: encoded_price.into(),
            sector_index: sector_index.to_le_bytes(),
        }
    }

    #[inline(always)]
    pub fn is_free(&self) -> bool {
        self.sector_index == LE_NIL
    }
}

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for UserOrderSectors {
    const LEN: usize = size_of::<PriceToIndex>() * (MAX_ORDERS * 2) as usize;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid.
        Ok(())
    }
}

const_assert_eq!(UserOrderSectors::LEN, size_of::<UserOrderSectors>());
const_assert_eq!(align_of::<UserOrderSectors>(), 1);

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for OrderSectors {
    const LEN: usize = size_of::<PriceToIndex>() * MAX_ORDERS as usize;

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid.
        Ok(())
    }
}

const_assert_eq!(OrderSectors::LEN, size_of::<OrderSectors>());
const_assert_eq!(align_of::<OrderSectors>(), 1);

// Safety:
//
// - Stable layout with `#[repr(C)]`.
// - `size_of` and `align_of` are checked below.
// - All bit patterns are valid.
unsafe impl Transmutable for PriceToIndex {
    const LEN: usize = size_of::<PriceToIndex>();

    #[inline(always)]
    fn validate_bit_patterns(_bytes: &[u8]) -> crate::error::DropsetResult {
        // All bit patterns are valid.
        Ok(())
    }
}

const_assert_eq!(PriceToIndex::LEN, size_of::<PriceToIndex>());
const_assert_eq!(align_of::<PriceToIndex>(), 1);

// -------------------------------------------------------------------------------------------------
// Create readable debug views for the encoded price to order sector mapping.
#[allow(dead_code)]
#[derive(Debug)]
struct PriceToIndexView {
    pub encoded_price: u32,
    pub sector_index: SectorIndex,
}

impl From<&PriceToIndex> for PriceToIndexView {
    fn from(value: &PriceToIndex) -> Self {
        Self {
            encoded_price: u32::from_le_bytes(value.encoded_price.as_array()),
            sector_index: SectorIndex::from_le_bytes(value.sector_index),
        }
    }
}

impl core::fmt::Debug for PriceToIndex {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let is_in_use = !self.is_free();
        let node: Option<PriceToIndexView> = is_in_use.then(|| self.into());
        write!(f, "{:#?}", node)
    }
}

// -------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use price::{
        to_biased_exponent,
        EncodedPrice,
        LeEncodedPrice,
        ValidatedPriceMantissa,
    };

    use crate::{
        error::DropsetError,
        state::{
            sector::{
                LeSectorIndex,
                SectorIndex,
                LE_NIL,
            },
            transmutable::Transmutable,
            user_order_sectors::{
                OrderSectors,
                PriceToIndex,
                UserOrderSectors,
                MAX_ORDERS,
            },
            U32_SIZE,
        },
    };

    extern crate std;

    #[test]
    fn new_all_free() {
        let order_sectors = UserOrderSectors::default();
        // All bids and asks should be free.
        assert!(order_sectors.asks.iter().all(|ask| ask.is_free()));
        assert!(order_sectors.bids.iter().all(|bid| bid.is_free()));
    }

    #[test]
    fn free_node_transmutable_bytes() {
        let free_bytes_vec = [[0; U32_SIZE], LE_NIL].concat();
        let free_bytes: &[u8; U32_SIZE * 2] = free_bytes_vec.as_slice().try_into().unwrap();
        let new_freed_from_transmute = PriceToIndex::load(free_bytes);
        assert!(new_freed_from_transmute.is_ok());
        let new_freed = new_freed_from_transmute.expect("Should transmute");
        assert!(new_freed.is_free());
        assert_eq!(new_freed.encoded_price.as_slice(), &[0u8; 4]);
        assert_eq!(new_freed.sector_index, LE_NIL);
        assert_eq!(new_freed, &PriceToIndex::new_free());
    }

    #[test]
    fn free_orders_transmutable_bytes() {
        let free_bytes_vec = [[0; U32_SIZE], LE_NIL].concat();
        let max_orders_all_freed: [u8; PriceToIndex::LEN * MAX_ORDERS as usize] = (0..MAX_ORDERS)
            .flat_map(|_| free_bytes_vec.iter().cloned())
            .collect::<std::vec::Vec<u8>>()
            .try_into()
            .unwrap();

        let new_max_orders_all_freed_from_transmute =
            OrderSectors::load(&max_orders_all_freed).expect("Should transmute");

        assert_eq!(
            new_max_orders_all_freed_from_transmute,
            &OrderSectors::default()
        );
    }

    #[test]
    fn happy_path_free_orders() {
        let order_sectors = UserOrderSectors::default();
        assert_eq!(order_sectors.bids, OrderSectors::default());
        assert_eq!(order_sectors.asks, OrderSectors::default());
    }

    #[test]
    fn happy_path_one_bid_one_ask() {
        let mut order_sectors = UserOrderSectors::default();
        let bid_encoded_price = EncodedPrice::new(
            to_biased_exponent!(1),
            ValidatedPriceMantissa::try_from(12_345_678).unwrap(),
        );
        let (bid_index, ask_index): (SectorIndex, SectorIndex) = (10, 11);
        let ask_encoded_price = EncodedPrice::new(
            to_biased_exponent!(2),
            ValidatedPriceMantissa::try_from(87_654_321).unwrap(),
        );
        let new_bid_price: &LeEncodedPrice = &bid_encoded_price.into();
        let new_ask_price: &LeEncodedPrice = &ask_encoded_price.into();

        order_sectors
            .bids
            .add(new_bid_price, &bid_index.to_le_bytes())
            .expect("Should add the mapping");
        order_sectors
            .asks
            .add(new_ask_price, &ask_index.to_le_bytes())
            .expect("Should add the mapping");
        assert_eq!(order_sectors.bids.get(new_bid_price).unwrap(), bid_index);
        assert_eq!(order_sectors.asks.get(new_ask_price).unwrap(), ask_index);
    }

    #[test]
    fn duplicate_bid_error() {
        let mut order_sectors = UserOrderSectors::default();
        let bid_encoded_price = EncodedPrice::new(
            to_biased_exponent!(1),
            ValidatedPriceMantissa::try_from(12_345_678).unwrap(),
        );
        let bid_index = 10u32;
        let bid_index_le_bytes = &bid_index.to_le_bytes();
        let bid_encoded_le_price: &LeEncodedPrice = &bid_encoded_price.into();
        order_sectors
            .bids
            .add(bid_encoded_le_price, bid_index_le_bytes)
            .expect("Should add the mapping");

        let failed_add = order_sectors
            .bids
            .add(bid_encoded_le_price, bid_index_le_bytes);

        assert!(matches!(
            failed_add,
            Err(DropsetError::OrderWithPriceAlreadyExists)
        ));
    }

    #[test]
    fn remove_nonexistent_order_error() {
        let mut order_sectors = UserOrderSectors::default();
        let bid_encoded_price = EncodedPrice::new(
            to_biased_exponent!(1),
            ValidatedPriceMantissa::try_from(12_345_678).unwrap(),
        );
        let failed_remove = order_sectors.bids.remove(bid_encoded_price.as_u32());
        assert!(matches!(failed_remove, Err(DropsetError::OrderNotFound)));
    }

    #[test]
    fn remove_order() {
        let mut order_sectors = UserOrderSectors::default();
        // All bids should be free.
        assert!(order_sectors.bids.iter().all(|bid| bid.is_free()));
        let bid_encoded_price = EncodedPrice::new(
            to_biased_exponent!(1),
            ValidatedPriceMantissa::try_from(12_345_678).unwrap(),
        );
        let bid_index = 10u32;
        assert!(order_sectors
            .bids
            .add(&bid_encoded_price.into(), &bid_index.to_le_bytes())
            .is_ok());
        // Count the number of orders that are in use (not free).
        let num_orders_in_use = order_sectors
            .bids
            .iter()
            .fold(0, |acc, p| match p.is_free() {
                true => acc,
                false => acc + 1,
            });
        assert_eq!(num_orders_in_use, 1);

        assert!(order_sectors
            .bids
            .remove(bid_encoded_price.as_u32())
            .is_ok());
        assert!(order_sectors.bids.iter().all(|bid| bid.is_free()));
    }

    #[test]
    fn too_many_orders_error() {
        let mut order_sectors = UserOrderSectors::default();
        for i in 0..=MAX_ORDERS as u32 {
            let encoded_price = EncodedPrice::new(
                to_biased_exponent!(0),
                ValidatedPriceMantissa::try_from(10_000_000 + i).unwrap(),
            );

            if i != MAX_ORDERS as u32 {
                // Add each new price to both bids and asks and assert it is successful.
                assert!(order_sectors
                    .bids
                    .add(&encoded_price.into(), &i.to_le_bytes())
                    .is_ok());
                assert!(order_sectors
                    .asks
                    .add(&encoded_price.into(), &i.to_le_bytes())
                    .is_ok());
            } else {
                // If this is the last order, it should fail, since it's one beyond the max amount.
                assert!(matches!(
                    order_sectors
                        .bids
                        .add(&encoded_price.into(), &i.to_le_bytes()),
                    Err(DropsetError::UserHasMaxOrders)
                ));
                assert!(matches!(
                    order_sectors
                        .asks
                        .add(&encoded_price.into(), &i.to_le_bytes()),
                    Err(DropsetError::UserHasMaxOrders)
                ));
            }
        }
    }

    #[test]
    fn repost_arbitrary_order() {
        let mut order_sectors = UserOrderSectors::default();
        let index_and_mantissa_pairs: [(u32, ValidatedPriceMantissa); MAX_ORDERS as usize] = [
            (1, ValidatedPriceMantissa::try_from(11_111_111).unwrap()),
            (2, ValidatedPriceMantissa::try_from(22_222_222).unwrap()),
            (3, ValidatedPriceMantissa::try_from(33_333_333).unwrap()),
            (4, ValidatedPriceMantissa::try_from(44_444_444).unwrap()),
            (5, ValidatedPriceMantissa::try_from(55_555_555).unwrap()),
        ];

        let index_and_encoded_price_pairs: [(u32, EncodedPrice); MAX_ORDERS as usize] =
            index_and_mantissa_pairs
                .into_iter()
                .map(|(i, mantissa)| (i, EncodedPrice::new(to_biased_exponent!(0), mantissa)))
                .collect::<std::vec::Vec<_>>()
                .try_into()
                .unwrap();

        for (i, encoded_price) in index_and_encoded_price_pairs.iter() {
            order_sectors
                .bids
                .add(&(*encoded_price).into(), &i.to_le_bytes())
                .unwrap();
        }

        // All bids should be in use.
        assert!(order_sectors.bids.iter().all(|bid| !bid.is_free()));

        let (old_sector_index, old_price) = *index_and_encoded_price_pairs.get(1).unwrap();

        let new_sector_index = 7;
        let new_mantissa = 77_777_777;
        let new_price = EncodedPrice::new(
            to_biased_exponent!(0),
            ValidatedPriceMantissa::try_from(new_mantissa).unwrap(),
        );

        // Ensure the new price doesn't exist in the bids yet.
        assert!(order_sectors.bids.get(&new_price.into()).is_none());

        // Ensure the old sector index doesn't equal the new index it's being updated to so the
        // final check is meaningful and not a misleading equality check.
        assert_ne!(old_sector_index, new_sector_index);

        // Remove the old price.
        assert!(order_sectors.bids.remove(old_price.as_u32()).is_ok());

        // Add the new price.
        assert!(order_sectors
            .bids
            .add(&new_price.into(), &new_sector_index.to_le_bytes())
            .is_ok());

        // Ensure the old price has been removed and the new price exists and is mapped to the new
        // sector index.
        assert!(order_sectors.bids.get(&old_price.into()).is_none());
        assert!(order_sectors.bids.get(&new_price.into()).is_some());
        assert_eq!(
            order_sectors.bids.get(&new_price.into()).unwrap(),
            new_sector_index
        );

        // Ensure there are no free bids.
        assert!(order_sectors.bids.iter().all(|bid| !bid.is_free()));

        // Check the final result in whole.
        let expected_index_and_encoded_price_pairs: [(u32, EncodedPrice); MAX_ORDERS as usize] = [
            (1, ValidatedPriceMantissa::try_from(11_111_111).unwrap()),
            (7, ValidatedPriceMantissa::try_from(77_777_777).unwrap()),
            (3, ValidatedPriceMantissa::try_from(33_333_333).unwrap()),
            (4, ValidatedPriceMantissa::try_from(44_444_444).unwrap()),
            (5, ValidatedPriceMantissa::try_from(55_555_555).unwrap()),
        ]
        .into_iter()
        .map(|(i, mantissa)| (i, EncodedPrice::new(to_biased_exponent!(0), mantissa)))
        .collect::<std::vec::Vec<_>>()
        .try_into()
        .unwrap();

        for (expected, result) in expected_index_and_encoded_price_pairs
            .iter()
            .zip(order_sectors.bids.iter())
        {
            let (expected_sector_index, expected_encoded_price): (&LeSectorIndex, &LeEncodedPrice) =
                (&expected.0.to_le_bytes(), &expected.1.into());
            assert_eq!(&result.sector_index, expected_sector_index);
            assert_eq!(&result.encoded_price, expected_encoded_price);
        }
    }
}
