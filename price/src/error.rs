#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(test, derive(strum_macros::Display))]
pub enum OrderInfoError {
    InvalidBaseExponent,
    InvalidQuoteExponent,
    ExponentUnderflow,
    ArithmeticOverflow,
    InvalidPriceMantissa,
    InvalidBiasedExponent,
    InfinityIsNotAFloat,
}
