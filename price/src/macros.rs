use static_assertions::const_assert_eq;

// Static assertions for macro invariants.
static_assertions::const_assert_eq!(crate::BIAS - 16, 0);
static_assertions::const_assert_eq!(crate::MAX_BIASED_EXPONENT, 31);

/// Documentation for [`pow10_u64`] relies on [`crate::BIAS`] == 16. If that changes,
/// [`crate::BIAS`] and the [`pow10_u64`] documentation needs to be updated.
const _: () = {
    const_assert_eq!(crate::BIAS, 16);
};

/// Performs base-10 exponentiation on a value using a biased exponent.
///
/// This facilitates representing negative exponent values with unsigned integers by ensuring the
/// biased exponent is never negative. The unbiased exponent is therefore the real exponent value.
///
/// # Parameters
/// - `$value`: The `u64` to be scaled by a power of 10.
/// - `$biased_exponent`: A biased exponent in the range `0..=31`.
///
/// # Biased Exponent Concept
/// The actual (aka unbiased) exponent is:
///
/// `exponent = $biased_exponent - price::BIAS`
///
/// With the current `BIAS = 16`, this means:
/// - `0`  → exponent `-16` (division by 10^16)
/// - `16` → exponent `0`   (multiplication by 1 aka 10^0)
/// - `31` → exponent `+15` (multiplication by 10^15)
///
/// The code output from the macro will error on an invalid biased exponent or arithmetic overflow.
///
/// # Reasoning behind exponent range
///
/// The decision to use a larger negative range instead of a larger positive range is because
/// a larger negative range results in the price mantissa * exponent product forming in a tighter
/// range around `1`.
///
/// For example, with `[-2, 1] vs [-1, 2]`:
///
/// ```markdown
/// # With [-2, 1] as the smallest/largest exponents
/// |                      | Smallest exponent   | Largest exponent    |
/// | -------------------- | ------------------- | ------------------- |
/// | Smallest mantissa    | 1.00 * 10^-2 = 0.01 | 1.00 * 10^1 =   10  |
/// | Largest mantissa     | 9.99 * 10^-2 = ~0.1 | 9.99 * 10^1 = ~100  |
/// | -------------------- | ------------------- | ------------------- |
/// ```
///
/// Both the smallest and largest products (0.01 and 100) are 2 orders
/// of magnitude below/above `1`.
///
/// ```markdown
/// # With [-1, 2] as the smallest/largest exponents
/// |                      | Smallest exponent  | Largest exponent     |
/// | -------------------- | ------------------ | -------------------- |
/// | Smallest mantissa    | 1.00 * 10^-1 = 0.1 | 1.00 * 10^2 =   100  |
/// | Largest mantissa     | 9.99 * 10^-1 =  ~1 | 9.99 * 10^2 = ~1000  |
/// | -------------------- | ------------------ | -------------------- |
/// ```
///
/// The lower product (0.1) is 1 order of magnitude below 1 and the higher
/// product (1000) is 3 orders of magnitude above 1.
///
/// The first option is preferable because it offers a more dynamic,
/// symmetrical range in terms of orders of magnitude below/above 1.
///
/// Therefore, [-16, 15] is used as the exponent range instead of [-15, 16].
///
#[macro_export]
#[rustfmt::skip]
macro_rules! pow10_u64 {
    ($value:expr, $biased_exponent:expr) => {{
        match $biased_exponent {
            /* BIAS - 16 */  0 => Ok($value / 10000000000000000u64),
            /* BIAS - 15 */  1 => Ok($value / 1000000000000000),
            /* BIAS - 14 */  2 => Ok($value / 100000000000000),
            /* BIAS - 13 */  3 => Ok($value / 10000000000000),
            /* BIAS - 12 */  4 => Ok($value / 1000000000000),
            /* BIAS - 11 */  5 => Ok($value / 100000000000),
            /* BIAS - 10 */  6 => Ok($value / 10000000000),
            /* BIAS - 9 */   7 => Ok($value / 1000000000),
            /* BIAS - 8 */   8 => Ok($value / 100000000),
            /* BIAS - 7 */   9 => Ok($value / 10000000),
            /* BIAS - 6 */  10 => Ok($value / 1000000),
            /* BIAS - 5 */  11 => Ok($value / 100000),
            /* BIAS - 4 */  12 => Ok($value / 10000),
            /* BIAS - 3 */  13 => Ok($value / 1000),
            /* BIAS - 2 */  14 => Ok($value / 100),
            /* BIAS - 1 */  15 => Ok($value / 10),
            /* BIAS - 0 */  16 => Ok($value),
            /* BIAS + 1 */  17 => checked_mul!($value, 10, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 2 */  18 => checked_mul!($value, 100, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 3 */  19 => checked_mul!($value, 1000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 4 */  20 => checked_mul!($value, 10000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 5 */  21 => checked_mul!($value, 100000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 6 */  22 => checked_mul!($value, 1000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 7 */  23 => checked_mul!($value, 10000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 8 */  24 => checked_mul!($value, 100000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 9 */  25 => checked_mul!($value, 1000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 10 */ 26 => checked_mul!($value, 10000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 11 */ 27 => checked_mul!($value, 100000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 12 */ 28 => checked_mul!($value, 1000000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 13 */ 29 => checked_mul!($value, 10000000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 14 */ 30 => checked_mul!($value, 100000000000000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 15 */ 31 => checked_mul!($value, 1000000000000000, OrderInfoError::ArithmeticOverflow),
            _ => Err(OrderInfoError::InvalidBiasedExponent),
        }
    }};
}

/// A checked subtraction with a custom error return value and the error path marked as cold.
///
/// *NOTE: This is only intended for usage with **unsigned** integer types.*
///
/// # Example
/// ```rust
/// enum MyError { BadSub }
///
/// let res: Result<u8, MyError> = price::checked_sub!(5u8, 4, MyError::BadSub);
/// assert!(matches!(res, Ok(1)));
///
/// let res: Result<u8, MyError> = price::checked_sub!(5u8, 6, MyError::BadSub);
/// assert!(matches!(res, Err(MyError::BadSub)));
/// ```
#[macro_export]
macro_rules! checked_sub {
    ($lhs:expr, $rhs:expr, $err:expr $(,)?) => {{
        let lhs = $lhs;
        let rhs = $rhs;
        if lhs >= rhs {
            // SAFETY: Just checked it will not underflow.
            unsafe { Ok(lhs.unchecked_sub(rhs)) }
        } else {
            ::pinocchio::hint::cold_path();
            Err($err)
        }
    }};
}

/// A checked multiplication with a custom error return value and the error path marked as cold.
///
/// *NOTE: This is only intended for usage with **unsigned** integer types.*
///
/// # Example
/// ```rust
/// enum MyError { BadMul }
///
/// let res: Result<u8, MyError> = price::checked_mul!(255u8, 1, MyError::BadMul);
/// assert!(matches!(res, Ok(255)));
///
/// let res: Result<u8, MyError> = price::checked_mul!(255u8, 2, MyError::BadMul);
/// assert!(matches!(res, Err(MyError::BadMul)));
/// ```
#[macro_export]
macro_rules! checked_mul {
    ($lhs:expr, $rhs:expr, $err:expr $(,)?) => {{
        match $lhs.checked_mul($rhs) {
            Some(val) => Ok(val),
            None => {
                ::pinocchio::hint::cold_path();
                Err($err)
            }
        }
    }};
}

/// Utility macro for converting unbiased exponents to biased exponents.
///
/// The input must be a literal or const value so that the const assertions work properly.
///
/// Requires the [`static_assertions`] library.
#[macro_export]
macro_rules! to_biased_exponent {
    ($unbiased_exponent:expr) => {{
        const UNBIASED: i16 = $unbiased_exponent as i16;
        ::static_assertions::const_assert!(UNBIASED >= $crate::UNBIASED_MIN);
        ::static_assertions::const_assert!(UNBIASED <= $crate::UNBIASED_MAX);
        (UNBIASED + $crate::BIAS as i16) as u8
    }};
}

#[cfg(test)]
mod tests {
    use crate::{
        OrderInfoError,
        BIAS,
        MAX_BIASED_EXPONENT,
        UNBIASED_MAX,
        UNBIASED_MIN,
    };

    #[test]
    fn check_max_biased_exponent() {
        // The max biased exponent should be valid.
        assert_eq!(
            pow10_u64!(2u64, MAX_BIASED_EXPONENT).unwrap(),
            2 * 10u64
                .checked_pow(MAX_BIASED_EXPONENT as u32 - BIAS as u32)
                .unwrap()
        );
        // One past the max biased exponent should result in an error.
        assert!(matches!(
            pow10_u64!(2u64, MAX_BIASED_EXPONENT + 1),
            Err(OrderInfoError::InvalidBiasedExponent)
        ));
    }

    #[test]
    fn unbiased_exponent_happy_paths() {
        let expected_min = (UNBIASED_MIN + BIAS as i16) as u8;
        assert_eq!(to_biased_exponent!(UNBIASED_MIN), expected_min);

        let expected_mid = BIAS;
        assert_eq!(to_biased_exponent!(0), expected_mid);

        let expected_max = (UNBIASED_MAX + BIAS as i16) as u8;
        assert_eq!(to_biased_exponent!(UNBIASED_MAX), expected_max);
    }
}
