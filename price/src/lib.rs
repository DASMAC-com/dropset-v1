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

#[allow(dead_code)]
/// The number of exponent bits is simply the remaining bits in a u32 after storing the price
/// mantissa bits.
const EXPONENT_BITS: u8 = U32_BITS - PRICE_MANTISSA_BITS;

#[allow(dead_code)]
/// The max biased exponent. This also determines the range of valid exponents.
/// I.e., 0 <= biased_exponent <= [`MAX_BIASED_EXPONENT`].
const MAX_BIASED_EXPONENT: u8 = (1 << (EXPONENT_BITS)) - 1;

/// [`BIAS`] is the number that satisfies: `BIAS + SMALLEST_POSSIBLE_EXPONENT == 0`.
/// That is, if the exponent range is 32 values from -16 <= n <= 15, the smallest possible exponent
/// is -16, so the BIAS must be 16.
/// Note the decision to use a larger negative range instead of a larger positive range (i.e.,
/// [-16, 15] instead of [-15, 16]) is because [-16, 15] has a tighter range in terms of the
/// difference in orders of magnitude for the smallest and largest exponent values.
pub const BIAS: u8 = 16;

/// The minimum unbiased exponent value.
#[cfg(test)]
const UNBIASED_MIN: i16 = 0 - BIAS as i16;
/// The maximum unbiased exponent value.
#[cfg(test)]
const UNBIASED_MAX: i16 = (BIAS as i16) - 1;

// Ensure that adding the bias to the max biased exponent never overflows.
static_assertions::const_assert!((MAX_BIASED_EXPONENT as u16) + (BIAS as u16) < (u8::MAX as u16));

/// The bitmask for the price mantissa calculated from the number of bits it uses.
pub const PRICE_MANTISSA_MASK: u32 = u32::MAX >> ((U32_BITS - PRICE_MANTISSA_BITS) as usize);

#[cfg(debug_assertions)]
mod debug_assertions {
    use static_assertions::*;

    use super::*;

    // The max price mantissa representable with `PRICE_MANTISSA_BITS` should exceed the upper bound
    // used to ensure a fixed number of digits for the price mantissa.
    const_assert!(MANTISSA_DIGITS_UPPER_BOUND < PRICE_MANTISSA_MASK);

    #[allow(dead_code)]
    /// The bitmask for the price exponent calculated from the number of bits in the price mantissa.
    pub const PRICE_EXPONENT_MASK: u32 = u32::MAX << (PRICE_MANTISSA_BITS as usize);

    // XOR'ing the price exponent and mantissa bit masks should result in a u32 with all 1 bits,
    // aka u32::MAX.
    const_assert_eq!(PRICE_EXPONENT_MASK ^ PRICE_MANTISSA_MASK, u32::MAX);
}

#[repr(C)]
#[cfg_attr(test, derive(Debug))]
pub struct OrderInfo {
    pub encoded_price: EncodedPrice,
    pub base_atoms: u64,
    pub quote_atoms: u64,
}

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
    // which means q - b may be negative and must be re-biased. Underflow only occurs if the
    // re-biased exponent difference is negative.
    let price_exponent_rebiased = checked_sub!(
        // Safety: The quote exponent must be <= MAX_BIASED_EXPONENT, and const assertions ensure
        // that `MAX_BIASED_EXPONENT + BIAS` is always less than `u8::MAX`.
        // Unit tests also guarantee this invariant.
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
    fn ensure_invalid_quote_exponent_fails() {
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
}
