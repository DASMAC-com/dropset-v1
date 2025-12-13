#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(test, derive(strum_macros::Display))]
pub enum OrderInfoError {
    ExponentUnderflow,
    ArithmeticOverflow,
    InvalidPriceMantissa,
    InvalidBiasedExponent,
    InfinityIsNotAFloat,
}
