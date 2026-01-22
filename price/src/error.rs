#[repr(u8)]
#[derive(Debug)]
#[cfg_attr(any(test, feature = "client"), derive(strum_macros::Display))]
pub enum OrderInfoError {
    ExponentUnderflow,
    ArithmeticOverflow,
    InvalidPriceMantissa,
    InvalidBiasedExponent,
    InfinityIsNotAFloat,
    AmountCannotBeZero,
}

#[cfg(feature = "client")]
impl std::error::Error for OrderInfoError {}
