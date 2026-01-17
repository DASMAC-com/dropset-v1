use pinocchio::hint;

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

const MAX_NORMALIZE_ITERATIONS: i16 = 100;

impl ValidatedPriceMantissa {
    /// Returns the validated price mantissa as a u32.
    #[inline(always)]
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    /// Normalize an f64 to a validated price mantissa and the power of 10 necessary to multiply 10
    /// by to go from the final price mantissa to the original price.
    ///
    /// For example, an input of 1.0 will return Some(10_000_000, -7)
    pub fn from_f64_with_normalize(
        nonnormalized_f64: f64,
    ) -> Result<(ValidatedPriceMantissa, i16), OrderInfoError> {
        if nonnormalized_f64.is_infinite() || nonnormalized_f64.is_nan() || nonnormalized_f64 <= 0.0
        {
            return Err(OrderInfoError::InvalidPriceMantissa);
        }

        let mut res = nonnormalized_f64;
        let mut pow: i16 = 0;

        while res < MANTISSA_DIGITS_LOWER_BOUND as f64 {
            res *= 10.0;
            pow -= 1;
            if pow < -MAX_NORMALIZE_ITERATIONS {
                return Err(OrderInfoError::InvalidPriceMantissa);
            }
        }

        // 99_999_999.99 is truncated down to 99_999_999, so instead of checking for
        // res > MANTISSA_DIGITS_UPPER_BOUND here, check for >= MANTISSA_*_BOUND + 1.
        while res >= (MANTISSA_DIGITS_UPPER_BOUND + 1) as f64 {
            res /= 10.0;
            pow += 1;
            if pow > MAX_NORMALIZE_ITERATIONS {
                return Err(OrderInfoError::InvalidPriceMantissa);
            }
        }

        Ok((Self(res as u32), pow))
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
    fn test_normalize_f64s() {
        let normalize = |value: f64| {
            ValidatedPriceMantissa::from_f64_with_normalize(value).map(|v| (v.0.as_u32(), v.1))
        };
        assert!(matches!(normalize(1.32), Ok((13_200_000, -7))));
        assert!(matches!(normalize(0.95123), Ok((95_123_000, -8))));
        assert!(matches!(normalize(123_456_789.0), Ok((12_345_678, 1))));
        assert!(matches!(normalize(78.12300001), Ok((78_123_000, -6))));
        assert!(normalize(0.000000).is_err());
        assert!(normalize(0.0).is_err());
        assert!(normalize(0 as f64).is_err());
        assert!(normalize(-1.0).is_err());
        assert!(normalize(-0.0000000000001).is_err());

        let tiny_number = normalize(0.000_000_000_000_012_345_678);
        assert!(matches!(tiny_number, Ok((12_345_678, -21))));

        // Imprecise floats means numbers like 0.0001 sometimes round to 0.000099999999 repeating.
        let tiny_imprecise_number = normalize(0.000_000_000_001);
        assert!(
            matches!(tiny_imprecise_number, Ok((10_000_000, -19)))
                || matches!(tiny_imprecise_number, Ok((99_999_999, -20)))
        );
    }
}
