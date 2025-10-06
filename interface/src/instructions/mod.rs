use pinocchio::program_error::ProgramError;

use crate::error::DropsetError;

pub mod amount;
pub mod close;
pub mod num_sectors;

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
pub enum InstructionTag {
    RegisterMarket,
    Deposit,
    Withdraw,
    Close,
    FlushEvents,
}

impl TryFrom<u8> for InstructionTag {
    type Error = ProgramError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            // SAFETY: A valid enum variant is guaranteed with the match pattern.
            // All variants are checked in the exhaustive instruction tag test.
            0..5 => Ok(unsafe { core::mem::transmute::<u8, Self>(value) }),
            _ => Err(DropsetError::InvalidInstructionTag.into()),
        }
    }
}

////////////////////
/// TODO: Add the load::<T>(data: &[u8]) -> &T      unsafe { *.as_ptr() as *const T } style
///       slice dereferencing with tags + enums.
///
/// that is, all fields would be 1-aligned slices, with getters/setters on each field's raw
/// pointer slice
///
/// const asserts on align-1 and size_of::<T>()
///
/// This couples the enum tag with the enum data in an intuitive manner
/// without having to copy *and* automatically avoids zero-init. Makes things much cleaner and
/// easier to iterate on.
///
/// with load::<T> passing ixn data around is ultimately just passing slice references around
///
/// it also means fewer checks on packing and unpacking because you can just do a single check
/// on the slice LEN as opposed to a check for each field
///
/// it would even be nice to have a proc macro for this:
///
/// ```
/// #[derive(Align1IxData)]
/// #[ix(tag = 3)] // Transfer
/// #[repr(C)]
/// pub struct TransferData {
///     // fields represented as slices of types with endianness must be private
///     // representing these as slices avoids alignment issues (dereferencing an unaligned u64 is
///     // UB)
///     amount: [u8; 8],
///     // only endian-agnostic values should be `pub` (and can thus forgo getters/setters)
///     pub decimals:  u8,
/// }
/// ```
#[cfg(test)]
mod tests {
    use super::InstructionTag;
    use strum::IntoEnumIterator;

    #[test]
    fn test_instruction_tag_from_u8_exhaustive() {
        for variant in InstructionTag::iter() {
            let variant_u8 = variant.clone() as u8;
            assert_eq!(
                InstructionTag::from_repr(variant_u8).unwrap(),
                InstructionTag::try_from(variant_u8).unwrap(),
            );
            assert_eq!(InstructionTag::try_from(variant_u8).unwrap(), variant);
        }
    }
}
