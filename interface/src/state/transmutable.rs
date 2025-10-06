// Derived from `pinocchio-token-interface` – commit 75116550519a9ee3fdfa6c819aca91e383fffa39, Apache-2.0.
// Substantial modifications by DASMAC, 2025:
// - Removed `Initializable` trait
// - Moved `load_*` functions to trait methods with validation contract
// - Added validate_bit_patterns requirement
// - Made load/load_mut safe for callers
// Original: https://github.com/solana-program/token/blob/75116550519a9ee3fdfa6c819aca91e383fffa39/p-interface/src/state/mod.rs

use crate::error::{DropsetError, DropsetResult};

/// Marker trait for a zero-copy view of bytes as `&Self` via an unchecked cast, aka a transmute.
///
/// # Safety
///
/// Implementor guarantees:
/// - `Self` has a stable layout; i.e. `#[repr(C)]` or `#[repr(transparent)]`
/// - `size_of::<Self> == LEN`
/// - `align_of::<Self> == 1`
/// - `validate_bit_patterns` returns `Ok(())` only when `bytes` is a valid representation of `Self`
pub unsafe trait Transmutable: Sized {
    /// The cumulative size in bytes of all fields in the struct.
    const LEN: usize;

    /// Validates that `bytes` represents a valid `Self`.
    ///
    /// Called after length checks, so implementors may assume `bytes.len() == Self::LEN`.
    /// Should be marked `#[inline(always)]` in implementations for optimal performance.
    fn validate_bit_patterns(bytes: &[u8]) -> DropsetResult;

    /// Returns a reference to `Self` from the given bytes after checking the byte length and
    /// validating that `bytes` represents a valid bit pattern.
    #[inline(always)]
    fn load(bytes: &[u8]) -> Result<&Self, DropsetError> {
        if bytes.len() != Self::LEN {
            return Err(DropsetError::InsufficientByteLength);
        }
        Self::validate_bit_patterns(bytes)?;

        // Safety: All bit patterns were validated and `bytes.len() == Self::LEN`
        unsafe { Ok(&*(bytes.as_ptr() as *const Self)) }
    }

    /// Returns a reference to `Self` from the given bytes.
    ///
    /// # Safety
    ///
    /// Caller guarantees either:
    /// - All bit patterns are valid for `Self`, *or*
    /// - `bytes` is a valid representation of `Self`; e.g. enum variants have been validated.
    ///
    /// And:
    /// - `bytes.len()` is equal to `Self::LEN`.
    #[inline(always)]
    unsafe fn load_unchecked(bytes: &[u8]) -> &Self {
        &*(bytes.as_ptr() as *const Self)
    }

    /// Returns a mutable reference to `Self` from the given bytes after checking the byte length
    /// and validating that `bytes` represents a valid bit pattern.
    #[inline(always)]
    fn load_mut(bytes: &mut [u8]) -> Result<&mut Self, DropsetError> {
        if bytes.len() != Self::LEN {
            return Err(DropsetError::InsufficientByteLength);
        }
        Self::validate_bit_patterns(bytes)?;

        // Safety: All bit patterns were validated and `bytes.len() == Self::LEN`
        unsafe { Ok(&mut *(bytes.as_ptr() as *mut Self)) }
    }

    /// Returns a mutable reference to `Self` from the given bytes.
    ///
    /// # Safety
    ///
    /// Caller guarantees either:
    /// - All bit patterns are valid for `Self`, *or*
    /// - `bytes` is a valid representation of `Self`; e.g. enum variants have been validated.
    ///
    /// And:
    /// - `bytes.len()` is equal to `Self::LEN`.
    #[inline(always)]
    unsafe fn load_unchecked_mut(bytes: &mut [u8]) -> &mut Self {
        &mut *(bytes.as_ptr() as *mut Self)
    }
}
