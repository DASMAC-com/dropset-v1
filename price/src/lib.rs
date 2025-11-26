/// The number of significant digits in the significand; i.e., the digits represented in the price.
pub const SIGNIFICANT_DIGITS: u8 = 9;

const MAX_SIGNIFICAND: u64 = 999_999_999;
const MIN_SIGNIFICAND: u64 = 100_000_000;

#[repr(C)]
pub struct Price {
    pub price: u64,
    pub base: u64,
    pub quote: u64,
}

#[derive(Debug)]
#[cfg_attr(test, derive(strum_macros::Display))]
pub enum PriceError {
    InvalidLotExponent,
    InvalidTickExponent,
    LotMinusTickUnderflow,
    ArithmeticOverflow,
    InvalidSignificand,
}

pub fn to_price(
    significand: u32,
    lots: u64,
    lot_exp: u8,
    tick_exp: u8,
) -> Result<Price, PriceError> {
    let significand = significand as u64;

    let base = lots
        .checked_mul(pow10_u64!(lot_exp, PriceError::InvalidLotExponent))
        .ok_or(PriceError::InvalidLotExponent)?;

    let significand_times_lots = significand
        .checked_mul(lots)
        .ok_or(PriceError::ArithmeticOverflow)?;

    let quote = significand_times_lots
        .checked_mul(pow10_u64!(tick_exp, PriceError::InvalidLotExponent))
        .ok_or(PriceError::ArithmeticOverflow)?;

    if lot_exp > tick_exp {
        return Err(PriceError::LotMinusTickUnderflow);
    }
    // Safety: the underflow condition was just checked; it returns early if underflow would occur.
    let price_exp = unsafe { tick_exp.unchecked_sub(lot_exp) };

    // Safety:
    // The mult operation here is always strictly <= (significand * lots) * tick_exp, since
    // price_exp == tick_exp - lot_exp.
    // This means that the multiplication operation here does not need to be checked, as it is
    // guaranteed to not overflow since the `quote` calculation did not overflow.
    let price =
        unsafe { significand.unchecked_mul(pow10_u64!(price_exp, PriceError::InvalidLotExponent)) };

    if (MIN_SIGNIFICAND..MAX_SIGNIFICAND).contains(&price) {
        Ok(Price { price, base, quote })
    } else {
        Err(PriceError::InvalidSignificand)
    }
}

/// Returns `10^exp` inline using a `match` on the exponent.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_simple_price() {
        let price = to_price(1234, 1, 0, 0).expect("Should calculate price");
        assert_eq!(price.base, 1);
        assert_eq!(price.quote, 1234);
        assert_eq!(price.price, 1234);
    }
}
