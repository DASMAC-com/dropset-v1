// Derived from `pinocchio-token-interface` – commit 75116550519a9ee3fdfa6c819aca91e383fffa39, Apache-2.0.
// Modifications by DASMAC, 2025. See: https://github.com/solana-program/token

use crate::error::DropsetError;

/// Marker trait for a zero-copy view of bytes as `&Self` via an unchecked cast
/// (e.g., `&*(bytes.as_ptr() as *const Self)`).
///
/// # Safety
/// **Implementor guarantees:**
/// - Use a stable layout (`#[repr(C)]` or `#[repr(transparent)]`) and ensure any
/// - `LEN` bytes form a valid `Self`. Prefer `[u8; N]` and/or transparent byte wrappers.
/// - `size_of::<Self> == LEN`
/// - `align_of::<Self> == 1`
///
/// **Caller guarantees:**
/// - The bytes represent a valid `Self`.
pub unsafe trait Transmutable: Sized {
    /// The cumulative size in bytes of all fields in the struct.
    const LEN: usize;
}

/// Returns a reference to a `T: Transmutable` from the given bytes after checking the byte length.
///
/// # Safety
/// - Caller must guarantee `bytes` is a valid representation of `T`.
#[inline(always)]
pub unsafe fn load<T: Transmutable>(bytes: &[u8]) -> Result<&T, DropsetError> {
    if bytes.len() != T::LEN {
        return Err(DropsetError::InsufficientByteLength);
    }
    Ok(&*(bytes.as_ptr() as *const T))
}

/// Returns a mutable reference to a `T: Transmutable` from the given bytes after checking the byte
/// length.
///
/// # Safety
/// - Caller must guarantee `bytes` is a valid representation of `T`.
#[inline(always)]
pub unsafe fn load_mut<T: Transmutable>(bytes: &mut [u8]) -> Result<&mut T, DropsetError> {
    if bytes.len() != T::LEN {
        return Err(DropsetError::InsufficientByteLength);
    }
    Ok(&mut *(bytes.as_ptr() as *mut T))
}
