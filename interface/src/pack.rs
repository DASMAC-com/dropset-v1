use core::mem::MaybeUninit;

use crate::state::transmutable::Transmutable;

pub const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();

/// # Safety
///
/// Implementor must guarantee `pack_into_slice` packs `LEN` bytes.
pub unsafe trait Pack<const LEN: usize>: Sized {
    /// Pack into a buffer of size LEN without zero initializing the buffer, then return the buffer.
    fn pack(&self) -> [u8; LEN] {
        let mut dst = [UNINIT_BYTE; LEN];
        self.pack_into_slice(&mut dst);

        // Safety: All LEN bytes were initialized in `pack_into_slice`.
        unsafe { *(dst.as_ptr() as *const [u8; LEN]) }
    }

    #[doc(hidden)]
    /// Pack into a destination slice of maybe uninitialized bytes of LEN length.
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; LEN]);
}

/// # Safety
///
/// Implementor guarantees:
/// - `size_of::<Self>() == LEN`
/// - `#[repr(C)]` or `#[repr(transparent)]`
/// - No padding between fields
/// - No invalid bit patterns for `Self`
pub unsafe trait AsSlice<const LEN: usize>: Sized {
    /// Returns `Self` as a referenced slice.
    #[inline(always)]
    fn as_slice(&self) -> &[u8; LEN] {
        unsafe { &*(self as *const Self as *const [u8; LEN]) }
    }
}

/// Safety: `Pack<LEN>` guarantees a length of `LEN`, and `Transmutable` guarantees a stable layout
/// with no padding or invalid bit patterns, so it's safe to provide a blanket implementation of
/// AsSlice for any type that implements both traits.
/// The `LEN` in pack should always match the `Transmutable::LEN`.
unsafe impl<T, const LEN: usize> AsSlice<LEN> for T where T: Pack<LEN> + Transmutable {}

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
