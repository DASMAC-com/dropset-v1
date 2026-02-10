use crate::{
    pack::ByteArray,
    Pack,
};

/// A trait for packing tagged structs into little-endian bytes with zero overhead. It exists
/// primarily to allow for more efficient, branch-friendly struct unpacking in Solana programs.
///
/// # Safety
///
/// Implementors must:
/// - Ensure [`write_bytes_tagged`](Tagged::write_bytes_tagged) writes at least
///   [`Tagged::LEN_WITH_TAG`] little-endian contiguous bytes to `dst`.
/// - Ensure [`Tagged::LEN_WITH_TAG`] equals [`Pack::LEN`] + 1.
/// - Not override [`Tagged::LEN_WITH_TAG`] to equal something other than the size and length of the
///   [`Tagged::PackedTagged`] array.
///
/// Callers of [`write_bytes_tagged`](Tagged::write_bytes_tagged) must:
/// - Ensure `dst` points to at least [`Tagged::LEN_WITH_TAG`] bytes of writable memory.
pub trait Tagged: Pack {
    /// Materialize the size of [`Tagged::PackedTagged`] for convenience purposes.
    ///
    /// # Safety
    ///
    /// This value must equal the size of [`Tagged::PackedTagged`].
    const LEN_WITH_TAG: usize = size_of::<Self::PackedTagged>();

    /// This is essentially just a `[u8; N]` but achieved without generics on the trait. See
    /// [`ByteArray`] for why it's necessary.
    ///
    /// # Safety
    ///
    /// This type must be `[u8; <Self as Pack>::LEN + 1]`.
    type PackedTagged: ByteArray;

    /// The tag byte; aka the discriminant.
    const TAG_BYTE: u8;

    /// Writes the [`Tagged::TAG_BYTE`] to `dst` at offset 0, then calls [`Pack::write_bytes`]
    /// starting at offset 1.
    ///
    /// # Safety
    ///
    /// `dst` must point to at least [`Tagged::LEN_WITH_TAG`] contiguous, writable bytes.
    #[inline(always)]
    unsafe fn write_bytes_tagged(&self, dst: *mut u8) {
        dst.write(Self::TAG_BYTE);
        <Self as Pack>::write_bytes(self, dst.add(1));
    }

    /// Initializes an array with `Self`'s packed, tagged bytes.
    fn pack_tagged(&self) -> Self::PackedTagged;
}
