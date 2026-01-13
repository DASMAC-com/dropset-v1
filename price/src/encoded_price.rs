use static_assertions::const_assert_eq;

use crate::{
    ValidatedPriceMantissa,
    PRICE_MANTISSA_BITS,
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
#[derive(Clone, Copy, Debug)]
pub struct EncodedPrice(u32);

pub const ENCODED_PRICE_INFINITY: u32 = u32::MAX;
pub const ENCODED_PRICE_ZERO: u32 = 0;

impl EncodedPrice {
    /// Creates a new [`EncodedPrice`] from a biased price exponent and a validated price mantissa.
    #[inline(always)]
    pub fn new(price_exponent_biased: u8, price_mantissa: ValidatedPriceMantissa) -> Self {
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
}

/// An [`EncodedPrice`] stored as little-endian bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeEncodedPrice([u8; U32_SIZE]);

impl LeEncodedPrice {
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8; U32_SIZE] {
        &self.0
    }

    #[inline(always)]
    pub fn zero() -> Self {
        Self::from(EncodedPrice::zero())
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
        to_biased_exponent,
        EncodedPrice,
        LeEncodedPrice,
        ValidatedPriceMantissa,
        BIAS,
        PRICE_MANTISSA_BITS,
        PRICE_MANTISSA_MASK,
    };

    #[test]
    fn encoded_price_mantissa_bits() {
        const EXPONENT: u8 = 0b0_1111;
        let price_mantissa = 0b000_1111_0000_1111_0000_1111_0000;
        let encoded_price = EncodedPrice::new(
            to_biased_exponent!(EXPONENT),
            ValidatedPriceMantissa::try_from(price_mantissa).unwrap(),
        );
        assert_eq!(
            encoded_price.0 >> PRICE_MANTISSA_BITS,
            (EXPONENT + BIAS) as u32
        );
        assert_eq!(encoded_price.0 & PRICE_MANTISSA_MASK, price_mantissa);
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
        let one = EncodedPrice::new(
            1,
            12_345_678.try_into().expect("Should be a valid mantissa"),
        );
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
}
