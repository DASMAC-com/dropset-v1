//! Utility functions to assist in calculating prices with decimals client-side. Not intended
//! to be used in smart contracts.

use core::num::NonZeroU64;

use rust_decimal::{
    dec,
    Decimal,
};

use crate::{
    OrderInfoArgs,
    OrderInfoError,
    ValidatedPriceMantissa,
    BIAS,
    UNBIASED_MAX,
    UNBIASED_MIN,
};

/// Try converting an unbiased exponent to a biased one.
pub fn try_to_biased_exponent(unbiased_exponent: i16) -> Result<u8, OrderInfoError> {
    if !(UNBIASED_MIN..=UNBIASED_MAX).contains(&unbiased_exponent) {
        return Err(OrderInfoError::InvalidBiasedExponent);
    }
    Ok((unbiased_exponent + BIAS as i16) as u8)
}

/// Returns the significant figures/digits in a u64 and the power of 10 to which that number must
/// be multiplied by to achieve the original input value.
fn get_sig_figs(value: NonZeroU64) -> (u64, i16) {
    let mut x = value.into();
    let mut pow: i16 = 0;
    while x % 10 == 0 {
        x /= 10;
        pow += 1;
    }

    (x, pow)
}

/// A helper function to convert a price ratio and order size (in base atoms) to order info args.
///
/// NOTE: The price must represent the atom price ratio since `price` represents the quote
/// atoms to base atoms price ratio.
pub fn to_order_info_args(
    price: Decimal,
    order_size_base_atoms: u64,
) -> Result<OrderInfoArgs, OrderInfoError> {
    let (validated_mantissa, price_exponent) = ValidatedPriceMantissa::try_into_with_scale(price)?;

    let order_size_non_zero =
        NonZeroU64::try_from(order_size_base_atoms).or(Err(OrderInfoError::AmountCannotBeZero))?;
    let (base_scalar, base_exponent_unbiased) = get_sig_figs(order_size_non_zero);

    // price_exponent == quote_exponent - base_exponent.
    // quote_exponent == price_exponent + base_exponent.
    let quote_exponent_unbiased = price_exponent + base_exponent_unbiased;

    let quote_exponent_biased = try_to_biased_exponent(quote_exponent_unbiased)?;
    let base_exponent_biased = try_to_biased_exponent(base_exponent_unbiased)?;

    Ok(OrderInfoArgs::new(
        validated_mantissa.as_u32(),
        base_scalar,
        base_exponent_biased,
        quote_exponent_biased,
    ))
}

pub fn decimal_pow10_i16(value: Decimal, pow: i16) -> Decimal {
    const TEN: Decimal = dec!(10);
    let is_negative = pow.is_negative();
    (0..pow.abs())
        .fold(
            value,
            |acc, _| {
                if is_negative {
                    acc / TEN
                } else {
                    acc * TEN
                }
            },
        )
        .normalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_biased_exponent;

    #[test]
    fn test_sig_figs() {
        assert_eq!(get_sig_figs(NonZeroU64::new(16801).unwrap()), (16801, 0));
        assert_eq!(get_sig_figs(NonZeroU64::new(168010).unwrap()), (16801, 1));
        assert_eq!(
            get_sig_figs(NonZeroU64::new(100_000_000_000).unwrap()),
            (1, 11)
        );
        assert_eq!(
            get_sig_figs(NonZeroU64::new(909_512_730_220).unwrap()),
            (90_951_273_022, 1)
        );
        assert_eq!(get_sig_figs(NonZeroU64::new(99).unwrap()), (99, 0));
        assert_eq!(get_sig_figs(NonZeroU64::new(909).unwrap()), (909, 0));
        assert_eq!(get_sig_figs(NonZeroU64::new(9090).unwrap()), (909, 1));
        assert_eq!(get_sig_figs(NonZeroU64::new(404_000).unwrap()), (404, 3));

        // Check that the values returned actually do equal the sig figs and power of 10.
        let n = NonZeroU64::new(4_125_900).unwrap();
        let expected_num: u64 = 41_259;
        let expected_pow_10: i16 = 2;
        assert_eq!(get_sig_figs(n), (expected_num, expected_pow_10));
        assert_eq!(n.get(), expected_num * 10u64.pow(expected_pow_10 as u32));
    }

    #[test]
    fn test_try_biased_exponents() {
        let expected_min = (UNBIASED_MIN + BIAS as i16) as u8;
        let expected_mid = BIAS;
        let expected_max = (UNBIASED_MAX + BIAS as i16) as u8;

        assert_eq!(try_to_biased_exponent(UNBIASED_MIN).unwrap(), expected_min);
        assert_eq!(try_to_biased_exponent(0).unwrap(), expected_mid);
        assert_eq!(try_to_biased_exponent(UNBIASED_MAX).unwrap(), expected_max);

        assert!(try_to_biased_exponent(UNBIASED_MIN - 1).is_err());
        assert!(try_to_biased_exponent(UNBIASED_MAX + 1).is_err());
    }

    #[test]
    fn test_to_order_info_args() {
        assert!(to_order_info_args(rust_decimal::dec!(1.5123), 500_000).is_ok());

        // Test the example in the doctest for the main to order info function.
        let base_atoms = 500 * 10u64.pow(6);
        let res = to_order_info_args(rust_decimal::dec!(1.25), base_atoms);
        let expected = (
            12_500_000,
            5,
            to_biased_exponent!(8),
            to_biased_exponent!(1),
        );
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), expected);
    }

    #[test]
    fn test_pow10_i16() {
        assert_eq!(decimal_pow10_i16(dec!(1.23), 2), dec!(123));
        assert_eq!(decimal_pow10_i16(dec!(1.6923), 3), dec!(1692.3));
        assert_eq!(decimal_pow10_i16(dec!(1.000333), 4), dec!(10003.33));
        assert_eq!(decimal_pow10_i16(dec!(1.23), -1), dec!(0.123));
        assert_eq!(decimal_pow10_i16(dec!(1.23), -2), dec!(0.0123));
        assert_eq!(decimal_pow10_i16(dec!(0.05123), -9), dec!(0.00000000005123));
    }
}
