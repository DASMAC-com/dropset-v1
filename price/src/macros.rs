/// Macro utility for calculating the value of an operation given a biased exponent, where a biased
/// exponent represents the base 10 positive or negative exponent value without using negative
/// values.
#[macro_export]
#[rustfmt::skip]
macro_rules! pow10_u64 {
    ($value:expr, $biased_exponent:expr) => {{
        ::static_assertions::const_assert_eq!($crate::BIAS - 16, 0);
        match $biased_exponent {
            /* BIAS - 16 */  0 => Ok($value / 10_000_000_000_000_000),
            /* BIAS - 15 */  1 => Ok($value / 1_000_000_000_000_000),
            /* BIAS - 14 */  2 => Ok($value / 100_000_000_000_000),
            /* BIAS - 13 */  3 => Ok($value / 10_000_000_000_000),
            /* BIAS - 12 */  4 => Ok($value / 1_000_000_000_000),
            /* BIAS - 11 */  5 => Ok($value / 100_000_000_000),
            /* BIAS - 10 */  6 => Ok($value / 10_000_000_000),
            /* BIAS - 9 */   7 => Ok($value / 1_000_000_000),
            /* BIAS - 8 */   8 => Ok($value / 100_000_000),
            /* BIAS - 7 */   9 => Ok($value / 10_000_000),
            /* BIAS - 6 */  10 => Ok($value / 1_000_000),
            /* BIAS - 5 */  11 => Ok($value / 100_000),
            /* BIAS - 4 */  12 => Ok($value / 10_000),
            /* BIAS - 3 */  13 => Ok($value / 1_000),
            /* BIAS - 2 */  14 => Ok($value / 100),
            /* BIAS - 1 */  15 => Ok($value / 10),
            /* BIAS - 0 */  16 => Ok($value),
            /* BIAS + 1 */  17 => checked_mul!($value, 10, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 2 */  18 => checked_mul!($value, 100, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 3 */  19 => checked_mul!($value, 1_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 4 */  20 => checked_mul!($value, 10_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 5 */  21 => checked_mul!($value, 100_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 6 */  22 => checked_mul!($value, 1_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 7 */  23 => checked_mul!($value, 10_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 8 */  24 => checked_mul!($value, 100_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 9 */  25 => checked_mul!($value, 1_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 10 */ 26 => checked_mul!($value, 10_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 11 */ 27 => checked_mul!($value, 100_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 12 */ 28 => checked_mul!($value, 1_000_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 13 */ 29 => checked_mul!($value, 10_000_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 14 */ 30 => checked_mul!($value, 100_000_000_000_000, OrderInfoError::ArithmeticOverflow),
            /* BIAS + 15 */ 31 => checked_mul!($value, 1_000_000_000_000_000, OrderInfoError::ArithmeticOverflow),
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
/// let res: Result<u8, MyError> = price::checked_sub!(5, 4, MyError::BadSub);
/// assert!(matches!(res, Ok(1)));
///
/// let res: Result<u8, MyError> = price::checked_sub!(5, 6, MyError::BadSub);
/// assert!(matches!(res, Err(MyError::BadSub)));
/// ```
#[macro_export]
macro_rules! checked_sub {
    ($lhs:expr, $rhs:expr, $err:expr $(,)?) => {{
        let lhs = $lhs;
        let rhs = $rhs;
        if lhs >= rhs {
            Ok(lhs - rhs)
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

/// Test utility macro for converting unbiased exponents to biased exponents.
#[cfg(test)]
#[macro_export]
macro_rules! to_biased_exponent {
    ($unbiased_exponent:expr) => {{
        let unbiased_signed = $unbiased_exponent as i16;
        match unbiased_signed {
            -15..=16 => (unbiased_signed + $crate::BIAS as i16) as u8,
            _ => panic!("Invalid unbiased exponent."),
        }
    }};
}
