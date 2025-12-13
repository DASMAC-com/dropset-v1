mod decoded_price;
mod encoded_price;
mod error;
mod macros;
mod validated_mantissa;

pub use decoded_price::*;
pub use encoded_price::*;
pub use error::*;
pub use validated_mantissa::*;

pub const MANTISSA_DIGITS_LOWER_BOUND: u32 = 10_000_000;
pub const MANTISSA_DIGITS_UPPER_BOUND: u32 = 99_999_999;

const U32_BITS: u8 = 32;
const PRICE_MANTISSA_BITS: u8 = 27;

/// The number of exponent bits is simply the remaining bits in a u32 after storing the price
/// mantissa bits.
#[allow(dead_code)]
const EXPONENT_BITS: u8 = U32_BITS - PRICE_MANTISSA_BITS;

/// The max biased exponent. This also determines the range of valid exponents.
/// I.e., 0 <= biased_exponent <= [`MAX_BIASED_EXPONENT`].
#[allow(dead_code)]
const MAX_BIASED_EXPONENT: u8 = (1 << (EXPONENT_BITS)) - 1;

/// [`BIAS`] is the number that satisfies: `BIAS + SMALLEST_POSSIBLE_EXPONENT == 0`.
/// It facilitates the expression of negative exponents with only unsigned integers.
///
/// The exponent range is 32 values from -16 <= n <= 15 and the smallest possible exponent
/// is -16, so the BIAS must be 16.
///
/// See [`pow10_u64`] for more information on the reasoning behind the exponent range.
pub const BIAS: u8 = 16;

/// The minimum unbiased exponent value.
#[cfg(test)]
const UNBIASED_MIN: i16 = 0 - BIAS as i16;
/// The maximum unbiased exponent value.
#[cfg(test)]
const UNBIASED_MAX: i16 = MAX_BIASED_EXPONENT as i16 - BIAS as i16;

// Ensure that adding the bias to the max biased exponent never overflows.
static_assertions::const_assert!((MAX_BIASED_EXPONENT as u16) + (BIAS as u16) <= (u8::MAX as u16));

/// The bitmask for the price mantissa calculated from the number of bits it uses.
pub const PRICE_MANTISSA_MASK: u32 = u32::MAX >> ((U32_BITS - PRICE_MANTISSA_BITS) as usize);

#[cfg(debug_assertions)]
mod debug_assertions {
    use static_assertions::*;

    use super::*;

    // The max price mantissa representable with `PRICE_MANTISSA_BITS` should exceed the upper bound
    // used to ensure a fixed number of digits for the price mantissa.
    const_assert!(MANTISSA_DIGITS_UPPER_BOUND < PRICE_MANTISSA_MASK);

    /// The bitmask for the price exponent calculated from the number of bits in the price mantissa.
    #[allow(dead_code)]
    pub const PRICE_EXPONENT_MASK: u32 = u32::MAX << (PRICE_MANTISSA_BITS as usize);

    // XOR'ing the price exponent and mantissa bit masks should result in a u32 with all 1 bits,
    // aka u32::MAX.
    const_assert_eq!(PRICE_EXPONENT_MASK ^ PRICE_MANTISSA_MASK, u32::MAX);
}

/// The fixed struct layout for information about a `dropset` order.
///
/// This struct is a C-style struct to facilitate a predictable, fixed layout for on-chain function
/// calls related to `dropset` orders.
#[repr(C)]
#[cfg_attr(test, derive(Debug))]
pub struct OrderInfo {
    /// The encoded price, containing an exponent and price mantissa.
    /// See [`EncodedPrice`] for more details.
    pub encoded_price: EncodedPrice,
    /// The indivisible units (aka atoms) of base token.
    pub base_atoms: u64,
    /// The indivisible units (aka atoms) of quote token.
    pub quote_atoms: u64,
}

/// # Safety note:
///
/// In the rebiased exponent calculation, there is an unchecked add that is actually safe. It's
/// safe because the prior function body ensures the quote exponent is <= MAX_BIASED_EXPONENT, and
/// const assertions ensure that `MAX_BIASED_EXPONENT + BIAS` is always <= `u8::MAX`.
///
/// [`tests::ensure_invalid_quote_exponent_fails_early`] ensures that the function fails early if
/// the quote exponent isn't <= MAX_BIASED_EXPONENT prior to the unchecked add.
pub fn to_order_info(
    price_mantissa: u32,
    base_scalar: u64,
    base_exponent_biased: u8,
    quote_exponent_biased: u8,
) -> Result<OrderInfo, OrderInfoError> {
    let validated_mantissa = ValidatedPriceMantissa::try_from(price_mantissa)?;

    let base_atoms = pow10_u64!(base_scalar, base_exponent_biased)?;

    let price_mantissa_times_base_scalar = checked_mul!(
        validated_mantissa.get() as u64,
        base_scalar,
        OrderInfoError::ArithmeticOverflow
    )?;

    let quote_atoms = pow10_u64!(price_mantissa_times_base_scalar, quote_exponent_biased)?;

    // Ultimately, the price mantissa is multiplied by:
    // 10 ^ (quote_exponent_biased - base_exponent_biased)
    // aka 10 ^ (q - b)
    // which means q - b may be negative and must be re-biased.
    //
    // Exponent underflow only occurs here if:
    //   `quote_exponent_biased + BIAS < base_exponent_biased`.
    let price_exponent_rebiased = checked_sub!(
        // Safety: See the function documentation.
        unsafe { quote_exponent_biased.unchecked_add(BIAS) },
        base_exponent_biased,
        OrderInfoError::ExponentUnderflow
    )?;

    Ok(OrderInfo {
        encoded_price: EncodedPrice::new(price_exponent_rebiased, validated_mantissa),
        base_atoms,
        quote_atoms,
    })
}

#[cfg(test)]
mod tests {
    use std::ops::Mul;

    use static_assertions::*;

    use super::*;

    #[test]
    fn happy_path_simple_price() {
        let base_biased_exponent = to_biased_exponent!(0);
        let quote_biased_exponent = to_biased_exponent!(-4);
        let order = to_order_info(12_340_000, 1, base_biased_exponent, quote_biased_exponent)
            .expect("Should calculate price");
        assert_eq!(order.base_atoms, 1);
        assert_eq!(order.quote_atoms, 1234);

        let decoded_price: f64 = DecodedPrice::try_from(order.encoded_price)
            .expect("Should decode")
            .try_into()
            .expect("Should be a valid f64");
        assert_eq!(decoded_price, "1234".parse().unwrap());
    }

    #[test]
    fn price_with_max_sig_digits() {
        let order = to_order_info(12345678, 1, to_biased_exponent!(0), to_biased_exponent!(0))
            .expect("Should calculate price");
        assert_eq!(order.base_atoms, 1);
        assert_eq!(order.quote_atoms, 12345678);

        let decoded_price: f64 = DecodedPrice::try_from(order.encoded_price)
            .expect("Should decode")
            .try_into()
            .expect("Should be a valid f64");
        assert_eq!(decoded_price, "12345678".parse().unwrap());
    }

    #[test]
    fn decimal_price() {
        let mantissa = 12345678;
        let order = to_order_info(mantissa, 1, to_biased_exponent!(8), to_biased_exponent!(0))
            .expect("Should calculate price");
        assert_eq!(order.quote_atoms, 12345678);
        assert_eq!(order.base_atoms, 100000000);

        let decoded_price = DecodedPrice::try_from(order.encoded_price).expect("Should decode");

        let (decoded_exponent, decoded_mantissa) = decoded_price
            .as_exponent_and_mantissa()
            .expect("Should be exponent + mantissa");
        let decoded_f64: f64 = decoded_price
            .clone()
            .try_into()
            .expect("Should be a valid f64");
        assert_eq!(decoded_mantissa.get(), mantissa);
        assert_eq!(decoded_f64, "0.12345678".parse().unwrap());
        assert_eq!(
            (decoded_mantissa.get() as f64).mul(10f64.powi(*decoded_exponent as i32 - BIAS as i32)),
            decoded_f64
        );
    }

    #[test]
    fn bias_ranges() {
        const_assert_eq!(16, BIAS);

        let val_156_e_neg_16: u64 = 1_560_000_000_000_000_000;
        let calculated = val_156_e_neg_16 / 10u64.pow(BIAS as u32);
        let expected = 156;
        assert_eq!(
            pow10_u64!(val_156_e_neg_16, 0).expect("0 is a valid biased exponent"),
            calculated,
        );
        assert_eq!(calculated, expected);

        let val: u64 = 156;
        let max_exponent = MAX_BIASED_EXPONENT as u32;
        let calculated = val
            * 10u64
                .checked_pow(max_exponent - BIAS as u32)
                .expect("Shouldn't overflow");
        let expected: u64 = 156_000_000_000_000_000;
        assert_eq!(
            pow10_u64!(val, max_exponent).expect("Exponent should be valid"),
            calculated
        );
        assert_eq!(calculated, expected);
    }

    #[test]
    fn ensure_price_mantissa_times_base_scalar_arithmetic_overflow() {
        const PRICE_MANTISSA: u32 = 10_000_000;

        assert!(to_order_info(
            PRICE_MANTISSA,
            u64::MAX / PRICE_MANTISSA as u64,
            to_biased_exponent!(0),
            to_biased_exponent!(0),
        )
        .is_ok());

        assert!(matches!(
            to_order_info(
                PRICE_MANTISSA + 1,
                u64::MAX / PRICE_MANTISSA as u64,
                to_biased_exponent!(0),
                to_biased_exponent!(0)
            ),
            Err(OrderInfoError::ArithmeticOverflow)
        ));
    }

    #[test]
    fn ensure_exponent_underflow() {
        let price_mantissa = 10_000_000;
        let base_scalar = 1;

        assert!(to_order_info(price_mantissa, base_scalar, BIAS, 0).is_ok());

        assert!(matches!(
            to_order_info(price_mantissa, base_scalar, BIAS + 1, 0),
            Err(OrderInfoError::ExponentUnderflow)
        ));
    }

    #[test]
    pub(crate) fn ensure_invalid_quote_exponent_fails_early() {
        let e_base = to_biased_exponent!(0);
        let e_quote = MAX_BIASED_EXPONENT + 1;

        // Ensure the base exponent is valid so that it can't be the trigger for the error.
        let _one_to_the_base_exponent = pow10_u64!(1u64, e_base).unwrap();

        let all_good = to_order_info(10_000_000, 1, e_base, e_base);
        let arithmetic_overflow = to_order_info(10_000_000, 1, e_base, e_quote - 1);
        let invalid_biased_exponent = to_order_info(10_000_000, 1, e_base, e_quote);

        assert!(all_good.is_ok());
        #[rustfmt::skip]
        assert!(matches!(arithmetic_overflow, Err(OrderInfoError::ArithmeticOverflow)));
        #[rustfmt::skip]
        assert!(matches!(invalid_biased_exponent, Err(OrderInfoError::InvalidBiasedExponent)));
    }

    #[test]
    fn max_and_max_plus_one_base() {
        let e_base = MAX_BIASED_EXPONENT;
        let e_quote = to_biased_exponent!(0);

        // Ensure the quote exponent is valid so that it can't be the trigger for the error.
        let _one_to_the_quote_exponent = pow10_u64!(1u64, e_quote).unwrap();

        let all_good = to_order_info(10_000_000, 1, e_base, e_quote);
        let invalid_quote_exponent = to_order_info(10_000_000, 1, e_base + 1, e_quote);

        assert!(all_good.is_ok());
        assert!(matches!(
            invalid_quote_exponent,
            Err(OrderInfoError::InvalidBiasedExponent)
        ));
    }

    #[test]
    fn quote_atoms_overflow() {
        let mantissa: u32 = 10_000_000;
        let base_scalar: u64 = 1;

        let quote_exponent = 12;
        assert!((mantissa as u64).checked_mul(base_scalar).is_some());

        // No overflow with quote exponent using core rust operations.
        assert!((mantissa as u64)
            .checked_mul(base_scalar)
            .unwrap()
            .checked_mul(10u64.checked_pow(quote_exponent as u32).unwrap())
            .is_some());

        // Overflow with quote exponent + 1 using core rust operations.
        assert!((mantissa as u64)
            .checked_mul(base_scalar)
            .unwrap()
            .checked_mul(10u64.checked_pow((quote_exponent + 1) as u32).unwrap())
            .is_none());

        // No overflow with quote exponent in `to_order_info`.
        assert!(to_order_info(
            mantissa,
            base_scalar,
            to_biased_exponent!(0),
            to_biased_exponent!(quote_exponent)
        )
        .is_ok());

        // Overflow with quote exponent + 1 in `to_order_info`.
        assert!(matches!(
            to_order_info(
                mantissa,
                base_scalar,
                to_biased_exponent!(0),
                to_biased_exponent!(quote_exponent + 1)
            ),
            Err(OrderInfoError::ArithmeticOverflow)
        ));
    }
}
