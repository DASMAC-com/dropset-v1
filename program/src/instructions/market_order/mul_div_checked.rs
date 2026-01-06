use core::num::{
    NonZeroU128,
    NonZeroU64,
};

use dropset_interface::error::DropsetError;

#[inline(always)]
pub fn mul_div_checked(
    multiplicand: u64,
    multiplier: u64,
    divisor: NonZeroU64,
) -> Result<u64, DropsetError> {
    let intermediate = price::checked_mul!(
        multiplicand as u128,
        multiplier as u128,
        DropsetError::ArithmeticOverflow
    )?;

    let res = intermediate / NonZeroU128::from(divisor);
    if res > u64::MAX as u128 {
        return Err(DropsetError::ArithmeticOverflow);
    }
    Ok(res as u64)
}
