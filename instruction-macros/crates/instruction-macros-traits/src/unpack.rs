use solana_address::Address;
use solana_program_error::ProgramError;

use crate::pack::Pack;

/// A trait for unpacking values from little-endian bytes with zero overhead.
///
/// [`Unpack::unpack`] is fallible because it errors when invalid byte patterns are used. For
/// example, with a `bool`, if the value is anything other than 0 or 1, [`Unpack::unpack`] will
/// return an error.
///
/// If there are no invalid byte patterns, [`Unpack::unpack`] should always return an `Ok(Self)`.
///
/// # Safety
///
/// Implementors must:
/// - Read at most [`Pack::LEN`] bytes from `src`.
///
/// Callers of [`unpack`](Unpack::unpack) must:
/// - Ensure `src` points to at least [`Pack::LEN`] bytes of readable memory.
pub unsafe trait Unpack: Pack + Sized {
    /// Unpacks `Self` from bytes at `src`.
    ///
    /// # Safety
    ///
    /// `src` must point to at least [`Pack::LEN`] bytes of readable memory.
    unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError>;
}

macro_rules! impl_unpack_uint {
    ($ty:ty) => {
        unsafe impl Unpack for $ty {
            #[inline(always)]
            unsafe fn unpack(src: *const u8) -> Result<$ty, ProgramError> {
                Ok(Self::from_le_bytes(
                    *(src as *const [u8; <$ty as Pack>::LEN]),
                ))
            }
        }
    };
}

/// # Safety
///
/// Reads exactly 1 byte from `src`.
unsafe impl Unpack for u8 {
    #[inline(always)]
    unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError> {
        Ok(src.read())
    }
}

/// # Safety
///
/// Reads exactly 1 byte from `src` and fails if the byte is not 0 or 1.
unsafe impl Unpack for bool {
    #[inline(always)]
    unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError> {
        match src.read() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

/// # Safety
///
/// Reads exactly 32 bytes from `src`.
unsafe impl Unpack for Address {
    #[inline(always)]
    unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError> {
        Ok(Address::new_from_array(*(src as *const [u8; Address::LEN])))
    }
}

impl_unpack_uint!(u16);
impl_unpack_uint!(u32);
impl_unpack_uint!(u64);
impl_unpack_uint!(u128);
