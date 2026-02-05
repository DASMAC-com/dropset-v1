use solana_address::Address;
use solana_program_error::ProgramError;

use crate::pack::Pack;

/// A trait for unpacking values from little-endian bytes with zero overhead.
///
/// [`Unpack::read_bytes`] is fallible because it errors when invalid byte patterns are used. For
/// example, with a `bool`, if the value is anything other than 0 or 1, [`Unpack::read_bytes`] will
/// return an error.
///
/// If there are no invalid byte patterns, [`Unpack::read_bytes`] should always return an
/// `Ok(Self)`.
///
/// # Safety
///
/// Implementors must:
/// - Read at most [`Pack::LEN`] bytes from `src`.
/// - Read from `src` as an unaligned pointer.
///
/// Callers of [`read_bytes`](Unpack::read_bytes) must:
/// - Ensure `src` points to at least [`Pack::LEN`] bytes of readable memory.
pub unsafe trait Unpack: Pack + Sized {
    /// Reads [`Pack::LEN`] bytes from `src` and constructs `Self` with them.
    ///
    /// Returns an error if the bytes at `src` represent an invalid byte pattern; e.g., a `bool`
    /// with a value of 2.
    ///
    /// # Safety
    ///
    /// Implementor guarantees:
    /// - At most [`Pack::LEN`] bytes are read from `src`.
    /// - `src` is read from as an unaligned pointer.
    ///
    /// Caller guarantees:
    /// - `src` points to at least [`Pack::LEN`] bytes of readable memory.
    unsafe fn read_bytes(src: *const u8) -> Result<Self, ProgramError>;

    /// Checks that the length of the passed slice is sufficient before reading its bytes, then
    /// creates a new [`Self`] from those bytes.
    fn unpack(data: &[u8]) -> Result<Self, ProgramError>;
}

macro_rules! impl_unpack_uint {
    ($ty:ty) => {
        unsafe impl Unpack for $ty {
            #[inline(always)]
            unsafe fn read_bytes(src: *const u8) -> Result<$ty, ProgramError> {
                Ok(Self::from_le_bytes(
                    *(src as *const [u8; <$ty as Pack>::LEN]),
                ))
            }

            #[inline(always)]
            fn unpack(data: &[u8]) -> Result<$ty, ProgramError> {
                if data.len() < <$ty as Pack>::LEN {
                    return Err(ProgramError::InvalidInstructionData);
                }

                unsafe { <$ty as Unpack>::read_bytes(data.as_ptr()) }
            }
        }
    };
}

/// # Safety
///
/// Reads exactly 1 byte from `src`.
unsafe impl Unpack for u8 {
    #[inline(always)]
    unsafe fn read_bytes(src: *const u8) -> Result<Self, ProgramError> {
        Ok(src.read())
    }

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Safety: `data` has at least 1 byte.
        unsafe { u8::read_bytes(data.as_ptr()) }
    }
}

/// # Safety
///
/// Reads exactly 1 byte from `src` and fails if the byte is not 0 or 1.
unsafe impl Unpack for bool {
    #[inline(always)]
    unsafe fn read_bytes(src: *const u8) -> Result<Self, ProgramError> {
        match src.read() {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.is_empty() {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Safety: `data` has at least 1 byte.
        unsafe { bool::read_bytes(data.as_ptr()) }
    }
}

/// # Safety
///
/// Reads exactly 32 bytes from `src`.
unsafe impl Unpack for Address {
    #[inline(always)]
    unsafe fn read_bytes(src: *const u8) -> Result<Self, ProgramError> {
        // Safety: The Address type is #[repr(transparent)] over [u8; 32] and align-1.
        Ok(*(src as *const Address))
    }

    #[inline(always)]
    fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        if data.len() < <Address as Pack>::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Safety: `data` has at least 32 bytes.
        unsafe { Address::read_bytes(data.as_ptr()) }
    }
}

impl_unpack_uint!(u16);
impl_unpack_uint!(u32);
impl_unpack_uint!(u64);
impl_unpack_uint!(u128);
