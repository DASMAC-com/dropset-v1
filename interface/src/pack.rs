use core::mem::MaybeUninit;

use crate::{
    error::DropsetError,
    state::{
        sector::{
            SectorIndex,
            NIL,
        },
        U16_SIZE,
        U32_SIZE,
        U64_SIZE,
    },
};

pub const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();

/// Writes bytes from a source slice into an uninitialized destination buffer.
///
/// This is a safe alternative to `ptr::copy_nonoverlapping` for writing to `MaybeUninit`
/// slices. The compiler should optimize this loop into a memcpy in release builds, providing
/// equivalent performance while avoiding `unsafe` and benefiting from compile-time bounds
/// checking on the slice operations.
///
/// # Safety considerations
/// Caller must ensure that `src.len()` is at least `dst.len()`. A partially written to `dst` is not
/// not immediate undefined behavior, but will cause UB if the slice pointer is later dereferenced
/// with an insufficiently sized array.
///
/// # Example
/// ```
/// use core::mem::MaybeUninit;
///
/// const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();
///
/// // Build a simple 5-byte message: [type, id, id, id, id]
/// let mut message = [UNINIT_BYTE; 5];
/// let message_type: u8 = 3;
/// let user_id: u32 = 1234;
///
/// // Write message type at offset 0
/// write_bytes(&mut message[0..1], &[message_type]);
/// // Write user ID at offset 1..5
/// write_bytes(&mut message[1..5], &user_id.to_le_bytes());
///
/// // This confines the `unsafe` behavior to the raw pointer cast back to a slice, which is now
/// // safe because all 5 bytes were explicitly written to.
/// let final_message: &[u8] = unsafe {
///     core::slice::from_raw_parts(message.as_ptr() as *const u8, 5)
/// };
/// ```
///
/// From pinocchio's `[no_std]` library:
/// <https://github.com/anza-xyz/pinocchio/blob/3044aaf5ea7eac01adc754d4bdf93c21c6e54d42/programs/token/src/lib.rs#L13>
#[inline(always)]
pub fn write_bytes(dst: &mut [MaybeUninit<u8>], src: &[u8]) {
    debug_assert_eq!(
        src.len(),
        dst.len(),
        "tried to `write_bytes` with mismatched src/dst lengths"
    );
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        d.write(*s);
    }
}

/// Safely unpacks a u16 from a slice of unknown length.
pub fn unpack_u16(instruction_data: &[u8]) -> Result<u16, DropsetError> {
    if instruction_data.len() >= U16_SIZE {
        // Safety: The instruction data is at least U16_SIZE bytes.
        Ok(unsafe { u16::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U16_SIZE])) })
    } else {
        Err(DropsetError::InvalidInstructionData)
    }
}

/// Safely unpacks a u32 from a slice of unknown length.
pub fn unpack_u32(instruction_data: &[u8]) -> Result<u32, DropsetError> {
    if instruction_data.len() >= U32_SIZE {
        // Safety: The instruction data is at least U32_SIZE bytes.
        Ok(unsafe { u32::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U32_SIZE])) })
    } else {
        Err(DropsetError::InvalidInstructionData)
    }
}

/// Safely unpacks a u64 from a slice of unknown length.
pub fn unpack_u64(instruction_data: &[u8]) -> Result<u64, DropsetError> {
    if instruction_data.len() >= U64_SIZE {
        // Safety: The instruction data is at least U64_SIZE bytes.
        Ok(unsafe { u64::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U64_SIZE])) })
    } else {
        Err(DropsetError::InvalidInstructionData)
    }
}

/// Safely unpacks a u64 and an optional sector index.
///
/// /// Sector indices passed by a caller can sometimes be optional, in which case `NIL` is used as
/// a `None`-like value. This function safely unpacks the u32 bytes into an Option<SectorIndex>.
///
/// This is useful because it means there's no need to use a COption type.
pub fn unpack_amount_and_optional_sector_index(
    instruction_data: &[u8],
) -> Result<(u64, Option<SectorIndex>), DropsetError> {
    if instruction_data.len() >= U64_SIZE + U32_SIZE {
        // Safety: The instruction data is at least U64_SIZE + U32_SIZE bytes.
        let (amount, index) = unsafe {
            let amount = u64::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U64_SIZE]));
            let index_bytes = *(instruction_data.as_ptr().add(U64_SIZE) as *const [u8; U32_SIZE]);
            (amount, u32::from_le_bytes(index_bytes))
        };
        let not_nil = index != NIL;
        let optional_index = not_nil.then_some(index);

        Ok((amount, optional_index))
    } else {
        Err(DropsetError::InvalidInstructionData)
    }
}

/// Safely unpacks a u64 and a sector index.
///
/// Note that the sector index returned could == `NIL`.
pub fn unpack_amount_and_sector_index(
    instruction_data: &[u8],
) -> Result<(u64, SectorIndex), DropsetError> {
    if instruction_data.len() >= U64_SIZE + U32_SIZE {
        // Safety: The instruction data is at least U64_SIZE + U32_SIZE bytes.
        Ok(unsafe {
            let amount = u64::from_le_bytes(*(instruction_data.as_ptr() as *const [u8; U64_SIZE]));
            let index_bytes = *(instruction_data.as_ptr().add(U64_SIZE) as *const [u8; U32_SIZE]);
            (amount, u32::from_le_bytes(index_bytes))
        })
    } else {
        Err(DropsetError::InvalidInstructionData)
    }
}
