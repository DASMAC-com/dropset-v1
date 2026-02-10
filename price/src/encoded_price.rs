use static_assertions::const_assert_eq;

use crate::{
    OrderInfoError,
    ValidatedPriceMantissa,
    PRICE_MANTISSA_BITS,
    PRICE_MANTISSA_MASK,
};

const U32_SIZE: usize = core::mem::size_of::<u32>();

/// The encoded price as a u32.
///
/// If `N` = the number of exponent bits and `M` = the number of price mantissa bits, the u32 bit
/// layout is:
///
/// ```text
///          N                M
/// |-------------------|-------------------|
///   [ exponent_bits ] | [ mantissa_bits ]
/// |---------------------------------------|
///                    32
/// ```
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EncodedPrice(u32);

pub const ENCODED_PRICE_INFINITY: u32 = u32::MAX;
pub const ENCODED_PRICE_ZERO: u32 = 0;

impl EncodedPrice {
    /// Creates a new [`EncodedPrice`] from a validated price mantissa and a biased price exponent.
    #[inline(always)]
    pub fn new(price_mantissa: ValidatedPriceMantissa, price_exponent_biased: u8) -> Self {
        // The biased price exponent doesn't need to be checked because a leftwards bitshift will
        // always discard irrelevant bits.
        let exponent_bits = (price_exponent_biased as u32) << PRICE_MANTISSA_BITS;

        // No need to mask the price mantissa since it has already been range checked/validated.
        // Thus it's guaranteed it will only occupy the lower M bits where M = PRICE_MANTISSA_BITS.
        Self(exponent_bits | price_mantissa.as_u32())
    }

    /// Returns the inner encoded price as a u32.
    #[inline(always)]
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// The encoded price representation of a market buy/taker order with no constraints on the
    /// maximum filled ask price.
    #[inline(always)]
    pub const fn infinity() -> Self {
        Self(ENCODED_PRICE_INFINITY)
    }

    #[inline(always)]
    pub fn is_infinity(&self) -> bool {
        self.0 == ENCODED_PRICE_INFINITY
    }

    /// The encoded price representation of a market sell/taker order with no constraints on the
    /// minimum filled bid price.
    #[inline(always)]
    pub const fn zero() -> Self {
        Self(ENCODED_PRICE_ZERO)
    }

    #[inline(always)]
    pub fn is_zero(&self) -> bool {
        self.0 == ENCODED_PRICE_ZERO
    }

    #[inline(always)]
    pub fn has_higher_ask_priority(&self, b: &Self) -> bool {
        self < b
    }

    #[inline(always)]
    pub fn has_higher_bid_priority(&self, b: &Self) -> bool {
        self > b
    }
}

#[cfg(any(feature = "client", debug_assertions))]
impl TryFrom<u32> for EncodedPrice {
    type Error = OrderInfoError;

    /// Convert a raw u32 to an [`EncodedPrice`] by unshifting the proper bits and revalidating
    /// the price mantissa.
    fn try_from(raw_value: u32) -> Result<Self, Self::Error> {
        let exponent_bits = raw_value >> PRICE_MANTISSA_BITS;
        let price_mantissa = raw_value & PRICE_MANTISSA_MASK;
        let res = Self::new(
            ValidatedPriceMantissa::try_from(price_mantissa)?,
            exponent_bits
                .try_into()
                .or(Err(OrderInfoError::InvalidBiasedExponent))?,
        );

        debug_assert_eq!(res.0, raw_value);

        Ok(res)
    }
}

/// An [`EncodedPrice`] stored as little-endian bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeEncodedPrice([u8; U32_SIZE]);

impl LeEncodedPrice {
    #[inline(always)]
    pub fn as_array(&self) -> [u8; U32_SIZE] {
        self.0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8; U32_SIZE] {
        &self.0
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self(EncodedPrice::zero().0.to_le_bytes())
    }
}

impl From<EncodedPrice> for LeEncodedPrice {
    #[inline(always)]
    fn from(value: EncodedPrice) -> Self {
        Self(value.0.to_le_bytes())
    }
}

const_assert_eq!(size_of::<EncodedPrice>(), U32_SIZE);
const_assert_eq!(size_of::<LeEncodedPrice>(), U32_SIZE);

#[cfg(test)]
mod tests {
    use crate::{
        encoded_price,
        EncodedPrice,
        LeEncodedPrice,
        BIAS,
        PRICE_MANTISSA_BITS,
        PRICE_MANTISSA_MASK,
    };

    #[test]
    fn encoded_price_mantissa_bits() {
        const EXPONENT: u8 = 0b0_1111;
        const PRICE_MANTISSA: u32 = 0b000_1111_0000_1111_0000_1111_0000;
        let encoded_price = encoded_price!(PRICE_MANTISSA, EXPONENT);
        assert_eq!(
            encoded_price.0 >> PRICE_MANTISSA_BITS,
            (EXPONENT + BIAS) as u32
        );
        assert_eq!(encoded_price.0 & PRICE_MANTISSA_MASK, PRICE_MANTISSA);
    }

    #[test]
    fn test_zero_and_infinity() {
        assert_eq!(EncodedPrice::infinity().0, u32::MAX);
        assert_eq!(EncodedPrice::zero().0, 0);
        assert!(EncodedPrice::infinity().is_infinity());
        assert!(EncodedPrice::zero().is_zero());
    }

    #[test]
    fn round_trip_encoded_to_le_encoded() {
        let zero = EncodedPrice::zero();
        let infinity = EncodedPrice::infinity();
        let one = encoded_price!(12_345_678, 1);
        let check_round_trip = |encoded: EncodedPrice| {
            let le_encoded_price = LeEncodedPrice::from(encoded);
            assert_eq!(le_encoded_price.as_slice(), &encoded.as_u32().to_le_bytes());
            assert_eq!(
                u32::from_le_bytes(*le_encoded_price.as_slice()),
                encoded.as_u32()
            );
        };
        check_round_trip(zero);
        check_round_trip(infinity);
        check_round_trip(one);
    }

    #[test]
    fn price_priority() {
        let price_1 = encoded_price!(10_000_000, 0);
        let price_2 = encoded_price!(20_000_000, 0);
        let price_3 = encoded_price!(30_000_000, 0);
        let price_4 = encoded_price!(40_000_000, 0);
        assert!(!price_1.has_higher_bid_priority(&price_2));
        assert!(!price_2.has_higher_bid_priority(&price_3));
        assert!(!price_3.has_higher_bid_priority(&price_4));

        assert!(price_4.has_higher_bid_priority(&price_3));
        assert!(price_3.has_higher_bid_priority(&price_2));
        assert!(price_2.has_higher_bid_priority(&price_1));

        assert!(price_1.has_higher_ask_priority(&price_2));
        assert!(price_2.has_higher_ask_priority(&price_3));
        assert!(price_3.has_higher_ask_priority(&price_4));

        assert!(!price_4.has_higher_ask_priority(&price_3));
        assert!(!price_3.has_higher_ask_priority(&price_2));
        assert!(!price_2.has_higher_ask_priority(&price_1));
    }
}
