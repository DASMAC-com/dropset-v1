use pinocchio::hint;
#[cfg(any(feature = "client", test))]
use rust_decimal::Decimal;

use crate::{
    OrderInfoError,
    MANTISSA_DIGITS_LOWER_BOUND,
    MANTISSA_DIGITS_UPPER_BOUND,
};
#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug))]
pub struct ValidatedPriceMantissa(u32);

impl TryFrom<u32> for ValidatedPriceMantissa {
    type Error = OrderInfoError;

    #[inline(always)]
    fn try_from(price_mantissa: u32) -> Result<Self, Self::Error> {
        if (MANTISSA_DIGITS_LOWER_BOUND..=MANTISSA_DIGITS_UPPER_BOUND).contains(&price_mantissa) {
            Ok(Self(price_mantissa))
        } else {
            hint::cold_path();
            Err(OrderInfoError::InvalidPriceMantissa)
        }
    }
}

impl ValidatedPriceMantissa {
    /// Returns the validated price mantissa as a u32.
    #[inline(always)]
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Try to convert a [`Decimal`] to a validated price mantissa and scale, where scale is defined
    /// as: `input_price = price_mantissa * 10^scale`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let price = rust_decimal::dec!(1.0);
    /// let (mantissa, scale) = price::ValidatedPriceMantissa::try_into_with_scale(price).unwrap();
    /// assert_eq!((mantissa.as_u32(), scale), (10_000_000, -7));
    /// ```
    #[cfg(any(feature = "client", test))]
    pub fn try_into_with_scale(
        price: Decimal,
    ) -> Result<(ValidatedPriceMantissa, i16), OrderInfoError> {
        const MAX_NORMALIZE_ITERATIONS: i16 = 100;

        if price.is_zero() || price.is_sign_negative() {
            return Err(OrderInfoError::InvalidPriceMantissa);
        }

        let mut res = price;
        let mut pow: i16 = 0;

        while res < Decimal::from(MANTISSA_DIGITS_LOWER_BOUND) {
            res *= Decimal::from(10);
            pow -= 1;
            if pow < -MAX_NORMALIZE_ITERATIONS {
                return Err(OrderInfoError::InvalidPriceMantissa);
            }
        }

        // 99_999_999.99 is truncated down to 99_999_999, so instead of checking for
        // res > MANTISSA_DIGITS_UPPER_BOUND here, check for >= MANTISSA_*_BOUND + 1.
        while res >= Decimal::from(MANTISSA_DIGITS_UPPER_BOUND + 1) {
            res /= Decimal::from(10);
            pow += 1;
            if pow > MAX_NORMALIZE_ITERATIONS {
                return Err(OrderInfoError::InvalidPriceMantissa);
            }
        }

        let validated_mantissa = Self(
            res.try_into()
                .map_err(|_| OrderInfoError::InvalidPriceMantissa)?,
        );

        Ok((validated_mantissa, pow))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_mantissas() {
        for mantissa in [
            MANTISSA_DIGITS_LOWER_BOUND,
            MANTISSA_DIGITS_LOWER_BOUND + 1,
            MANTISSA_DIGITS_UPPER_BOUND,
            MANTISSA_DIGITS_UPPER_BOUND - 1,
        ] {
            let validated_mantissa = ValidatedPriceMantissa::try_from(mantissa);
            assert!(validated_mantissa.is_ok());
            assert_eq!(validated_mantissa.unwrap().0, mantissa);
        }
    }

    #[test]
    fn invalid_mantissas() {
        assert!(matches!(
            ValidatedPriceMantissa::try_from(MANTISSA_DIGITS_LOWER_BOUND - 1),
            Err(OrderInfoError::InvalidPriceMantissa)
        ));
        assert!(matches!(
            ValidatedPriceMantissa::try_from(MANTISSA_DIGITS_UPPER_BOUND + 1),
            Err(OrderInfoError::InvalidPriceMantissa)
        ));
    }

    #[test]
    fn test_normalize_values() {
        use rust_decimal::dec;

        let check = |value: Decimal, expected: (u32, i16)| {
            let res =
                ValidatedPriceMantissa::try_into_with_scale(value).map(|v| (v.0.as_u32(), v.1));
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), expected);
        };

        check(dec!(1.32), (13_200_000, -7));
        check(dec!(0.95123), (95_123_000, -8));
        check(dec!(123_456_789.0), (12_345_678, 1));
        check(dec!(78.12300001), (78_123_000, -6));
        check(dec!(0.000_000_000_000_012_345_678), (12_345_678, -21));
        check(dec!(0.000_000_000_001), (10_000_000, -19));

        assert!(ValidatedPriceMantissa::try_into_with_scale(dec!(0.000000)).is_err());
        assert!(ValidatedPriceMantissa::try_into_with_scale(dec!(0.0)).is_err());
        assert!(ValidatedPriceMantissa::try_into_with_scale(Decimal::ZERO).is_err());
        assert!(ValidatedPriceMantissa::try_into_with_scale(dec!(-1.0)).is_err());
        assert!(ValidatedPriceMantissa::try_into_with_scale(dec!(-0.0000000000001)).is_err());
    }
}
