#![no_std]

#[cfg(any(feature = "client", test))]
extern crate std;

#[cfg(any(feature = "client", test))]
mod decoded_price;

#[cfg(any(feature = "client", test))]
pub use decoded_price::*;
#[cfg(any(feature = "client", test))]
pub mod client_helpers;

mod encoded_price;
mod error;
mod macros;
mod validated_mantissa;

pub use encoded_price::*;
pub use error::*;
use instruction_macros::{
    Pack,
    Unpack,
};
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

/// The minimum unbiased exponent value. Primarily for usage in tests and client contexts.
pub const UNBIASED_MIN: i16 = 0 - BIAS as i16;

/// The maximum unbiased exponent value. Primarily for usage in tests and client contexts.
pub const UNBIASED_MAX: i16 = MAX_BIASED_EXPONENT as i16 - BIAS as i16;

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
#[derive(Debug, Clone)]
pub struct OrderInfo {
    /// The encoded price, containing an exponent and price mantissa.
    /// See [`EncodedPrice`] for more details.
    pub encoded_price: EncodedPrice,
    /// The indivisible units (aka atoms) of base token.
    pub base_atoms: u64,
    /// The indivisible units (aka atoms) of quote token.
    pub quote_atoms: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Pack, Unpack, PartialEq, Eq)]
pub struct OrderInfoArgs {
    /// The price mantissa.
    pub price_mantissa: u32,
    /// The scalar for the base token.
    pub base_scalar: u64,
    /// The biased base exponent.
    pub base_exponent_biased: u8,
    /// The biased quote exponent.
    pub quote_exponent_biased: u8,
}

impl OrderInfoArgs {
    #[inline(always)]
    pub fn new(
        price_mantissa: u32,
        base_scalar: u64,
        base_exponent_biased: u8,
        quote_exponent_biased: u8,
    ) -> Self {
        Self {
            price_mantissa,
            base_scalar,
            base_exponent_biased,
            quote_exponent_biased,
        }
    }

    /// Creates [`Self`] with no exponent scaling; i.e., unbiased base and quote exponents are zero.
    #[inline(always)]
    pub fn new_unscaled(price_mantissa: u32, base_scalar: u64) -> Self {
        Self {
            price_mantissa,
            base_scalar,
            base_exponent_biased: biased_exponent!(0),
            quote_exponent_biased: biased_exponent!(0),
        }
    }
}

impl From<(u32, u64, u8, u8)> for OrderInfoArgs {
    #[inline(always)]
    fn from(value: (u32, u64, u8, u8)) -> Self {
        OrderInfoArgs::new(value.0, value.1, value.2, value.3)
    }
}

/// Convert order inputs into a serializable, non-decimalized [`OrderInfo`].
///
/// This function accepts a **price mantissa**, a **base scalar**, and **biased base/quote
/// exponents**, and produces an order whose amounts are fully expressed in atomic units.
///
/// This function is primarily intended to be used on-chain. If your intention is to go from a
/// decimal price to order instruction data, use [`crate::client_helpers::to_order_info_args`] to
/// get the input args to this function.
///
/// # Example
///
/// The following example shows how to place an order for 500 EUR at a price of 1.25 USD / 1 EUR.
///
/// Typically, stablecoins use 6 decimals in their SPL-token implementations, so 6 decimals are used
/// in this example as well. In this example, EUR and USD are stablecoins on-chain representing
/// their corresponding currencies. This means that:
///
/// - 1 EUR = 10^6 atoms
/// - 1 USD = 10^6 atoms
///
/// The price mantissa stores significant digits within a fixed range `10_000_000 ..= 99_999_999`.
///
/// If the price is 1.25 USD/EUR, the significant digits are 125, and the price mantissa is thus:
///
/// ```text
/// price_mantissa = 12_500_000
/// ```
///
/// The rest of the input values can be determined as follows:
///
/// ```rust
/// use price;
/// use static_assertions::{const_assert_eq};
///
/// const PRICE_MANTISSA: u64 = 12_500_000;
/// // 500 EUR worth of base atoms is 500 * 10^6.
/// const BASE_ATOMS: u64 = 500 * 10u64.pow(6);
/// // Derive the base scalar similarly to how the price mantissa is derived: using the sig figs.
/// // Since the intended number of base atoms is 500_000_000, the only sig fig is 5.
/// const BASE_SCALAR: u64 = 5;
/// // The unbiased base exponent is simply the power of 10 to which you multiply the base scalar
/// // by to get to the base atoms:
/// const BASE_EXPONENT_UNBIASED: u8 = 8;
/// const_assert_eq!(BASE_ATOMS, BASE_SCALAR * 10u64.pow(BASE_EXPONENT_UNBIASED as u32));
///
/// // The quote atoms can be derived from the price and base atoms. The price is 1.25 USD / 1 EUR,
/// // which can also cleanly translate to multiplying by 125 / 100.
/// const QUOTE_ATOMS: u64 = BASE_ATOMS * 125 / 100;
///
/// // The (unbiased) quote exponent can be cleanly derived from the price, the price mantissa, and
/// // the already derived unbiased base exponent:
/// //
/// // price = price_mantissa / 10 ^ (price_exponent);
/// // where
/// // price_exponent = quote_exponent_unbiased - base_exponent_unbiased
/// //
/// // There's a difference of magnitude 7 between 1.25 and 12_500_000. Count the digits to see:
/// //  1 234 567
/// // 12_500_000
/// //
/// // Thus the price_exponent = -7, so:
/// // price_exponent = quote_exponent_unbiased - base_exponent_unbiased
/// // -7 = quote_exponent_unbiased - 8
/// // quote_exponent_unbiased = 1
/// const QUOTE_EXPONENT_UNBIASED: u8 = 1;
/// const_assert_eq!(QUOTE_ATOMS, PRICE_MANTISSA * BASE_SCALAR * 10u64.pow(QUOTE_EXPONENT_UNBIASED as u32));
///
/// let args: price::OrderInfoArgs = (
///     PRICE_MANTISSA as u32,
///     BASE_SCALAR,
///     price::biased_exponent!(BASE_EXPONENT_UNBIASED),
///     price::biased_exponent!(QUOTE_EXPONENT_UNBIASED),
/// ).into();
/// let order = price::to_order_info(args).expect("Should create order info");
///
/// let derived_price = order.quote_atoms as f64 / order.base_atoms as f64;
///
/// assert_eq!(order.base_atoms, BASE_ATOMS);
/// assert_eq!(order.quote_atoms, QUOTE_ATOMS);
/// assert_eq!(derived_price, 1.25);
/// ```
///
/// # Safety
///
/// This function performs an unchecked add when rebiasing the price exponent. This is safe because:
///
/// - The quote exponent is validated to be `<= MAX_BIASED_EXPONENT`
/// - Compile-time assertions guarantee `MAX_BIASED_EXPONENT + BIAS <= u8::MAX`
///
/// The test [`tests::ensure_invalid_quote_exponent_fails_early`] ensures invalid inputs
/// are rejected before the unchecked operation.
#[allow(rustdoc::broken_intra_doc_links)]
pub fn to_order_info(args: OrderInfoArgs) -> Result<OrderInfo, OrderInfoError> {
    let OrderInfoArgs {
        price_mantissa,
        base_scalar,
        base_exponent_biased,
        quote_exponent_biased,
    } = args;
    let validated_mantissa = ValidatedPriceMantissa::try_from(price_mantissa)?;

    let base_atoms = pow10_u64!(base_scalar, base_exponent_biased)?;

    let price_mantissa_times_base_scalar = checked_mul!(
        validated_mantissa.as_u32() as u64,
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
        encoded_price: EncodedPrice::new(validated_mantissa, price_exponent_rebiased),
        base_atoms,
        quote_atoms,
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use rust_decimal::{
        dec,
        Decimal,
    };
    use static_assertions::*;

    use super::*;
    use crate::client_helpers::decimal_pow10_i16;

    #[test]
    fn happy_path_simple_price() {
        let base_biased_exponent = biased_exponent!(0);
        let quote_biased_exponent = biased_exponent!(-4);
        let order =
            to_order_info((12_340_000, 1, base_biased_exponent, quote_biased_exponent).into())
                .expect("Should calculate price");
        assert_eq!(order.base_atoms, 1);
        assert_eq!(order.quote_atoms, 1234);

        let decoded_price: Decimal = DecodedPrice::try_from(order.encoded_price)
            .expect("Should decode")
            .try_into()
            .expect("Should be a valid Decimal");
        assert_eq!(decoded_price, dec!(1234));
    }

    #[test]
    fn price_with_max_sig_digits() {
        let order = to_order_info(OrderInfoArgs::new_unscaled(12345678, 1))
            .expect("Should calculate price");
        assert_eq!(order.base_atoms, 1);
        assert_eq!(order.quote_atoms, 12345678);

        let decoded_price: Decimal = DecodedPrice::try_from(order.encoded_price)
            .expect("Should decode")
            .try_into()
            .expect("Should be a valid Decimal");
        assert_eq!(decoded_price, dec!(12345678));
    }

    #[test]
    fn decimal_price() {
        let mantissa = 12345678;
        let order = to_order_info(OrderInfoArgs::new(
            mantissa,
            1,
            biased_exponent!(8),
            biased_exponent!(0),
        ))
        .expect("Should calculate price");
        assert_eq!(order.quote_atoms, 12345678);
        assert_eq!(order.base_atoms, 100000000);

        let decoded_price = DecodedPrice::try_from(order.encoded_price).expect("Should decode");

        std::println!("{decoded_price:#?}");

        let (decoded_exponent, decoded_mantissa) = decoded_price
            .as_exponent_and_mantissa()
            .expect("Should be exponent + mantissa");
        let decoded: Decimal = decoded_price
            .clone()
            .try_into()
            .expect("Should be a valid Decimal");
        assert_eq!(decoded_mantissa.as_u32(), mantissa);
        assert_eq!(decoded, dec!(0.12345678));
        let unbiased_exponent = *decoded_exponent as i16 - BIAS as i16;
        assert_eq!(
            decimal_pow10_i16(Decimal::from(decoded_mantissa.as_u32()), unbiased_exponent),
            decoded
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
    fn check_unscaled_args_output() {
        let price_mantissa = 12_345_678;
        let base_scalar = 87_654_321;
        assert_eq!(
            OrderInfoArgs::new_unscaled(price_mantissa, base_scalar),
            OrderInfoArgs::new(
                price_mantissa,
                base_scalar,
                biased_exponent!(0),
                biased_exponent!(0)
            )
        );
    }

    #[test]
    fn ensure_price_mantissa_times_base_scalar_arithmetic_overflow() {
        const PRICE_MANTISSA: u32 = 10_000_000;

        assert!(to_order_info(OrderInfoArgs::new_unscaled(
            PRICE_MANTISSA,
            u64::MAX / PRICE_MANTISSA as u64,
        ))
        .is_ok());

        assert!(matches!(
            to_order_info(OrderInfoArgs::new_unscaled(
                PRICE_MANTISSA + 1,
                u64::MAX / PRICE_MANTISSA as u64,
            )),
            Err(OrderInfoError::ArithmeticOverflow)
        ));
    }

    #[test]
    fn ensure_exponent_underflow() {
        let price_mantissa = 10_000_000;
        let base_scalar = 1;

        assert!(to_order_info(OrderInfoArgs::new(price_mantissa, base_scalar, BIAS, 0)).is_ok());

        assert!(matches!(
            to_order_info(OrderInfoArgs::new(price_mantissa, base_scalar, BIAS + 1, 0)),
            Err(OrderInfoError::ExponentUnderflow)
        ));
    }

    #[test]
    pub(crate) fn ensure_invalid_quote_exponent_fails_early() {
        let e_base = biased_exponent!(0);
        let e_quote = MAX_BIASED_EXPONENT + 1;

        // Ensure the base exponent is valid so that it can't be the trigger for the error.
        let _one_to_the_base_exponent = pow10_u64!(1u64, e_base).unwrap();

        let all_good = to_order_info((10_000_000, 1, e_base, e_base).into());
        let arithmetic_overflow = to_order_info((10_000_000, 1, e_base, e_quote - 1).into());
        let invalid_biased_exponent = to_order_info((10_000_000, 1, e_base, e_quote).into());

        assert!(all_good.is_ok());
        #[rustfmt::skip]
        assert!(matches!(arithmetic_overflow, Err(OrderInfoError::ArithmeticOverflow)));
        #[rustfmt::skip]
        assert!(matches!(invalid_biased_exponent, Err(OrderInfoError::InvalidBiasedExponent)));
    }

    #[test]
    fn max_and_max_plus_one_base() {
        let e_base = MAX_BIASED_EXPONENT;
        let e_quote = biased_exponent!(0);

        // Ensure the quote exponent is valid so that it can't be the trigger for the error.
        let _one_to_the_quote_exponent = pow10_u64!(1u64, e_quote).unwrap();

        let all_good = to_order_info((10_000_000, 1, e_base, e_quote).into());
        let invalid_quote_exponent = to_order_info((10_000_000, 1, e_base + 1, e_quote).into());

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

        const QUOTE_EXPONENT_UNBIASED: i32 = 12;
        assert!((mantissa as u64).checked_mul(base_scalar).is_some());

        // No overflow with quote exponent using core rust operations.
        assert!((mantissa as u64)
            .checked_mul(base_scalar)
            .unwrap()
            .checked_mul(10u64.checked_pow(QUOTE_EXPONENT_UNBIASED as u32).unwrap())
            .is_some());

        // Overflow with quote exponent + 1 using core rust operations.
        assert!((mantissa as u64)
            .checked_mul(base_scalar)
            .unwrap()
            .checked_mul(
                10u64
                    .checked_pow((QUOTE_EXPONENT_UNBIASED + 1) as u32)
                    .unwrap()
            )
            .is_none());

        // No overflow with quote exponent in `to_order_info`.
        assert!(to_order_info(OrderInfoArgs::new(
            mantissa,
            base_scalar,
            biased_exponent!(0),
            biased_exponent!(QUOTE_EXPONENT_UNBIASED)
        ))
        .is_ok());

        // Overflow with quote exponent + 1 in `to_order_info`.
        assert!(matches!(
            to_order_info(OrderInfoArgs::new(
                mantissa,
                base_scalar,
                biased_exponent!(0),
                biased_exponent!(QUOTE_EXPONENT_UNBIASED + 1)
            )),
            Err(OrderInfoError::ArithmeticOverflow)
        ));
    }
}
