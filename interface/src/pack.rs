use core::mem::MaybeUninit;

pub const UNINIT_BYTE: MaybeUninit<u8> = MaybeUninit::uninit();

pub trait Pack<const LEN: usize>: Sized {
    /// Pack into a buffer of size LEN without zero initializing the buffer, then return the buffer.
    fn pack(&self) -> [u8; LEN] {
        let mut dst = [UNINIT_BYTE; LEN];
        self.pack_into_slice(&mut dst);

        // Safety: All LEN bytes were initialized in `pack_into_slice`.
        unsafe { *(dst.as_ptr() as *const [u8; LEN]) }
    }

    /// Returns a byte array view of `self`.
    ///
    /// # Safety
    /// Caller must ensure `Self` satisfies:
    /// - `size_of::<Self>() == LEN`
    /// - `#[repr(C)]` or `#[repr(transparent)]`
    /// - No padding between fields
    /// - All bit patterns are valid for `Self`
    unsafe fn as_bytes(&self) -> &[u8; LEN] {
        unsafe { &*(self as *const Self as *const [u8; LEN]) }
    }

    #[doc(hidden)]
    /// Pack into a destination slice of maybe uninitialized bytes of LEN length.
    fn pack_into_slice(&self, dst: &mut [MaybeUninit<u8>; LEN]);
}

/// A byte-by-byte copy from one slice to another without having to zero init on the `dst` slice.
/// This is more explicit and less efficient than `sol_memcpy_` (in non-solana land it would be
/// `copy_from_nonoverlapping`), but it removes the risk of undefined behavior since the iterator
/// makes it impossible to write past the end of `dst`.
///
/// While it's not technically undefined behavior, a partially written to `dst` will result in
/// unexpected results. Ensure that both slices are at least the expected length.
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
/// https://github.com/anza-xyz/pinocchio/blob/3044aaf5ea7eac01adc754d4bdf93c21c6e54d42/programs/token/src/lib.rs#L13`
#[inline(always)]
pub fn write_bytes(dst: &mut [MaybeUninit<u8>], src: &[u8]) {
    for (d, s) in dst.iter_mut().zip(src.iter()) {
        d.write(*s);
    }
}
