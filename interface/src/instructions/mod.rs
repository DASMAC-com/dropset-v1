use pinocchio::program_error::ProgramError;

use crate::error::DropsetError;

pub mod close_seat;
pub mod deposit;
pub mod flush_events;
pub mod register_market;
pub mod withdraw;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(test, derive(strum_macros::FromRepr, strum_macros::EnumIter))]
pub enum InstructionTag {
    RegisterMarket,
    Deposit,
    Withdraw,
    CloseSeat,
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

#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;

    use super::InstructionTag;

    #[test]
    fn test_instruction_tag_from_u8_exhaustive() {
        for variant in InstructionTag::iter() {
            let variant_u8 = variant as u8;
            assert_eq!(
                InstructionTag::from_repr(variant_u8).unwrap(),
                InstructionTag::try_from(variant_u8).unwrap(),
            );
            assert_eq!(InstructionTag::try_from(variant_u8).unwrap(), variant);
        }
    }
}
