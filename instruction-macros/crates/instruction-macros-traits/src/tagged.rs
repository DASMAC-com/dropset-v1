use crate::Pack;

pub trait Tagged: Pack {
    /// The length of the struct including the tag byte. This should be [`<Self as Pack>::LEN`] + 1.
    const LEN_WITH_TAG: usize;

    /// The tag byte; aka the discriminant.
    const TAG_BYTE: u8;

    /// Writes the `Self::TAG_BYTE` to `dst` at offset 0, then calls `<Self as Pack>::write_bytes`
    /// starting at offset 1.
    ///
    /// # Safety
    ///
    /// Caller must guarantee `dst` has [`<Self as Pack>::LEN`] + 1 writable bytes.
    unsafe fn write_bytes_tagged(&self, dst: *mut u8) {
        dst.write(Self::TAG_BYTE);
        <Self as Pack>::write_bytes(self, dst.add(1));
    }
}
