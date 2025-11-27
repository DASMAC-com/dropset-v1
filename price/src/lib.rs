use pinocchio::hint;

const MANTISSA_DIGITS_LOWER_BOUND: u32 = 10_000_000;
const MANTISSA_DIGITS_UPPER_BOUND: u32 = 99_999_999;

const PRICE_MANTISSA_BITS: u8 = 27;

/// The base-10 bias of the exponents passed to price functions.
/// # Example
/// // Say you want to divide some value by
const BIAS: u8 = 15;
// TODO: wip- finish  the bias implementation here/above/in the pow10_* calls
const BIAS_: u8 = 1 << (32 - PRICE_MANTISSA_BITS);

#[cfg(debug_assertions)]
mod debug_assertions {
    use static_assertions::*;

    use super::*;

    const U32_BITS: usize = 32;

    /// The bitmask for the price exponent calculated from the number of bits in the price mantissa.
    pub const PRICE_EXPONENT_MASK: u32 = u32::MAX << (PRICE_MANTISSA_BITS as usize);

    /// The bitmask for the price mantissa calculated from the number of bits it uses.
    pub const PRICE_MANTISSA_MASK: u32 = u32::MAX >> (U32_BITS - PRICE_MANTISSA_BITS as usize);

    // The max price mantissa is its bitmask. Ensure the mantissa's upper bound doesn't exceed that.
    const_assert!(MANTISSA_DIGITS_UPPER_BOUND < PRICE_MANTISSA_MASK);

    // The price exponent and mantissa bit masks xor'd should just be a u32 with all hi bits.
    const_assert_eq!(PRICE_EXPONENT_MASK ^ PRICE_MANTISSA_MASK, u32::MAX);
}

#[derive(Copy, Clone, Debug)]
/// The encoded price as a u32.
///
/// If `N` = the number of exponent bits and `M` = the number of price bits, the u32 bit layout is:
///
/// ```text
///          N                M
/// |-----------------|--------------|
/// [ exponent_bits ] | [ price_bits ]
/// |--------------------------------|
///                 32
/// ```
pub struct EncodedPrice(pub u32);

impl EncodedPrice {
    #[inline(always)]
    pub fn new(price_exponent: u8, price_mantissa: ValidatedPriceMantissa) -> Self {
        let exponent_bits = (price_exponent as u32) << PRICE_MANTISSA_BITS;

        // No need to mask the price mantissa since it has already been range checked/validated.
        // Thus it will only occupy the lower M bits where M = PRICE_MANTISSA_BITS.
        Self(exponent_bits | price_mantissa.0)
    }
}

pub struct ValidatedPriceMantissa(pub u32);

impl TryFrom<u32> for ValidatedPriceMantissa {
    type Error = OrderInfoError;

    #[inline(always)]
    fn try_from(price_mantissa: u32) -> Result<Self, Self::Error> {
        if (MANTISSA_DIGITS_LOWER_BOUND..MANTISSA_DIGITS_UPPER_BOUND).contains(&price_mantissa) {
            Ok(Self(price_mantissa))
        } else {
            hint::cold_path();
            Err(OrderInfoError::InvalidPriceMantissa)
        }
    }
}

#[repr(C)]
pub struct OrderInfo {
    pub price: EncodedPrice,
    pub base_atoms: u64,
    pub quote_atoms: u64,
}

#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(test, derive(strum_macros::Display))]
pub enum OrderInfoError {
    InvalidBaseExponent,
    InvalidQuoteExponent,
    BaseMinusQuoteUnderflow,
    ArithmeticOverflow,
    InvalidPriceMantissa,
    InvalidBiasedExponent,
}

pub fn to_order_info(
    price_mantissa: u32,
    base_scalar: u64,
    base_exponent_with_bias: u8,
    quote_exponent_with_bias: u8,
) -> Result<OrderInfo, OrderInfoError> {
    let b_biased = base_exponent_with_bias;
    let q_biased = quote_exponent_with_bias;
    let b_checked_bias =
        checked_sub_unsigned!(b_biased, BIAS, OrderInfoError::InvalidBiasedExponent)?;
    let biased_q = checked_sub_unsigned!(q_biased, BIAS, OrderInfoError::InvalidBiasedExponent)?;

    // let base = checked_mul_unsigned!(base_scalar, )

    // TODO: Implement bias for these values (these values represent bias factored in)
    // 1: this means making sure these checks are correct (the checks below)
    // 2: and factoring in negative exponents in the pow10 calculations..?

    let lot_exp = base_exponent_with_bias
        .checked_sub(BIAS)
        .ok_or(OrderInfoError::InvalidBiasedExponent)?;
    let tick_exp = quote_exponent_with_bias
        .checked_sub(BIAS)
        .ok_or(OrderInfoError::InvalidBiasedExponent)?;

    let base = base_scalar
        .checked_mul(pow10_u64!(lot_exp, OrderInfoError::InvalidBaseExponent))
        .ok_or(OrderInfoError::InvalidBaseExponent)?;

    let significand_times_lots = price_mantissa
        .checked_mul(base_scalar)
        .ok_or(OrderInfoError::ArithmeticOverflow)?;

    let quote = significand_times_lots
        .checked_mul(pow10_u64!(tick_exp, OrderInfoError::InvalidBaseExponent))
        .ok_or(OrderInfoError::ArithmeticOverflow)?;

    if lot_exp > tick_exp {
        hint::cold_path();
        return Err(OrderInfoError::BaseMinusQuoteUnderflow);
    }
    // Safety: the underflow condition was just checked; it returns early if underflow would occur.
    let price_exponent = unsafe { tick_exp.unchecked_sub(lot_exp) };

    let validated_mantissa = ValidatedPriceMantissa::try_from(price_mantissa)?;

    Ok(OrderInfo {
        price: EncodedPrice::new(price_exponent, validated_mantissa),
        base_atoms,
        quote_atoms,
    })
}

/// Returns `10^exp` inline using a `match` on the exponent.
///
/// The `exp`
///
/// Supported exponents map directly to their corresponding power-of-ten
/// value. Any unsupported exponent causes the macro to emit an early
/// `return Err($err)` from the surrounding function.
///
/// # Example
///
/// ```
/// let scale = pow10_u64!(3, MyError::InvalidExponent);
/// assert_eq!(scale, 1000); // 10^3
/// ```
#[macro_export]
macro_rules! pow10_u64 {
    ($exp:expr, $err:expr) => {{
        match $exp {
            0 => 1u64,
            1 => 10,
            2 => 100,
            3 => 1_000,
            4 => 10_000,
            5 => 100_000,
            6 => 1_000_000,
            this needs to be 0 - 15 for negative exponents and then 15 - 31 for positive exponents
            needs to factor in bias
            7 => 10_000_000,
            8 => 100_000_000,
            9 => 1_000_000_000,
            10 => 10_000_000_000,
            11 => 100_000_000_000,
            12 => 1_000_000_000_000,
            13 => 10_000_000_000_000,
            14 => 100_000_000_000_000,
            15 => 1_000_000_000_000_000,
            _ => return Err($err),
        }
    }};
}

/// A checked subtraction with a custom error return value and the error path marked as cold.
///
/// This is only intended for usage with **unsigned** integer types.
///
/// # Example
/// ```rust
/// let res: Result<u8, MyError> = checked_sub!(5, 4, MyError::BadSub);
/// assert_eq!(res, Ok(1));
///
/// let res: Result<u8, MyError> = checked_sub!(5, 6, MyError::BadSub);
/// assert_eq!(res, Err(MyError::BadSub));
/// ```
#[macro_export]
macro_rules! checked_sub_unsigned {
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
/// This is only intended for usage with **unsigned** integer types.
///
/// # Example
/// ```rust
/// let res: Result<u8, MyError> = checked_mul!(255, 1, MyError::BadMul);
/// assert_eq!(res, Ok(255));
///
/// let res: Result<u8, MyError> = checked_mul!(255, 2, MyError::BadMul);
/// assert_eq!(res, Err(MyError::BadMul));
/// ```
#[macro_export]
macro_rules! checked_mul_unsigned {
    ($lhs:expr, $rhs:expr, $err:expr $(,)?) => {{
        let lhs = $lhs;
        let rhs = $rhs;
        match lhs.checked_mul(rhs) {
            Some(val) => Ok(val),
            None => {
                ::pinocchio::hint::cold_path();
                Err($err)
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_simple_price() {
        let price = to_order_info(1234, 1, 0, 0).expect("Should calculate price");
        assert_eq!(price.base_atoms, 1);
        assert_eq!(price.quote_atoms, 1234);
        assert_eq!(price.price, 1234);
    }

    #[test]
    fn hi_encoded_price_bits() {}
}
