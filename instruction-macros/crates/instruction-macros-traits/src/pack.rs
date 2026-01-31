use solana_address::Address;
/// A trait for packing values into little-endian bytes with zero overhead.
///
/// # Safety
///
/// Implementors must:
/// - Ensure [`write_bytes`](Pack::write_bytes) writes at least [`Pack::LEN`] little-endian
///   contiguous bytes to `dst`.
/// - Not override the default implementation of [`Pack::LEN`], which is simply the size of the
///   [`Pack::Packed`] array.
///
/// Callers of [`write_bytes`](Pack::write_bytes) must:
/// - Ensure `dst` points to at least [`Pack::LEN`] bytes of writable memory.
pub unsafe trait Pack {
    /// # Safety
    ///
    /// Do not override this value. This must equal `size_of::<Self::Packed>()`.
    const LEN: usize = size_of::<Self::Packed>();

    /// This is essentially just a `[u8; N]` but without having to use generics on the trait.
    ///
    /// See [`ByteArray`] for why it's necessary.
    type Packed: ByteArray;

    /// Writes `self` into bytes at `dst`.
    ///
    /// # Safety
    ///
    /// `dst` must point to at least [`Pack::LEN`] bytes of writable memory.
    unsafe fn write_bytes(&self, dst: *mut u8);

    /// Initializes an array with `Self`'s packed bytes.
    fn pack(&self) -> Self::Packed;
}

/// A helper macro for implementing Pack on unsigned integers.
macro_rules! impl_pack_uint {
    ($ty:ty) => {
        unsafe impl Pack for $ty {
            type Packed = [u8; Self::LEN];

            const LEN: usize = core::mem::size_of::<$ty>();

            #[inline(always)]
            unsafe fn write_bytes(&self, dst: *mut u8) {
                core::ptr::copy_nonoverlapping(self.to_le_bytes().as_ptr(), dst, Self::LEN);
            }

            #[inline(always)]
            fn pack(&self) -> [u8; Self::LEN] {
                self.to_le_bytes()
            }
        }
    };
}

/// # Safety
///
/// Writes exactly 1 byte to `dst`.
unsafe impl Pack for u8 {
    type Packed = [u8; 1];

    #[inline(always)]
    unsafe fn write_bytes(&self, dst: *mut u8) {
        dst.write(*self);
    }

    #[inline(always)]
    fn pack(&self) -> Self::Packed {
        [*self]
    }
}

impl_pack_uint!(u16);
impl_pack_uint!(u32);
impl_pack_uint!(u64);
impl_pack_uint!(u128);

/// # Safety
///
/// Writes exactly 1 byte to `dst`.
unsafe impl Pack for bool {
    type Packed = [u8; 1];

    #[inline(always)]
    unsafe fn write_bytes(&self, dst: *mut u8) {
        dst.write(*self as u8)
    }

    #[inline(always)]
    fn pack(&self) -> Self::Packed {
        [*self as u8]
    }
}

/// # Safety
///
/// Writes exactly 32 bytes to `dst`.
unsafe impl Pack for Address {
    type Packed = [u8; size_of::<Address>()];

    #[inline(always)]
    unsafe fn write_bytes(&self, dst: *mut u8) {
        core::ptr::copy_nonoverlapping(self as *const Address as *const u8, dst, Self::LEN);
    }

    #[inline(always)]
    fn pack(&self) -> Self::Packed {
        self.to_bytes()
    }
}

/// All of this ceremony (for [`private::Sealed`] and [`ByteArray`]) is to facilitate requiring
/// a fixed-size array in the [`Pack`] implementation *without* the usage of generics.
///
/// It's nice to not have to use generics in the trait because `impl Pack<1> for u8` is very odd,
/// since you could also implement Pack<2> for it and it would make no sense.
///
/// With this sealed trait, implementors of [`Pack`] must provide a statically sized `[u8; N]` array
/// type, which is then used in the return type and for calculating the overall length when packed.
mod private {
    pub trait Sealed {}
    impl<const N: usize> Sealed for [u8; N] {}
}

pub trait ByteArray: private::Sealed + AsRef<[u8]> + AsMut<[u8]> {}

impl<const N: usize> ByteArray for [u8; N] {}
