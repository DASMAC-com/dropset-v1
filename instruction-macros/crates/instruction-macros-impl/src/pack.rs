use solana_address::Address;
use solana_program_error::ProgramError;

/// A trait for packing values into little-endian bytes with zero overhead.
///
/// # Safety
///
/// Implementors must:
/// - Write exactly [`Self::LEN`] little-endian bytes to `dst` when [`pack`](Pack::pack) is called.
///
/// Callers of [`pack`](Pack::pack) must:
/// - Ensure `dst` points to at least [`Self::LEN`] bytes of writable memory.
pub unsafe trait Pack {
    /// The number of bytes this type packs into.
    const LEN: usize;

    /// Packs `self` into bytes at `dst`.
    ///
    /// # Safety
    ///
    /// `dst` must point to at least [`Self::LEN`] bytes of writable memory.
    unsafe fn pack(&self, dst: *mut u8);
}

/// A helper macro for implementing Pack on unsigned integers.
macro_rules! impl_pack_uint {
    ($ty:ty) => {
        unsafe impl Pack for $ty {
            const LEN: usize = core::mem::size_of::<$ty>();

            #[inline(always)]
            unsafe fn pack(&self, dst: *mut u8) {
                core::ptr::copy_nonoverlapping(self.to_le_bytes().as_ptr(), dst, Self::LEN);
            }
        }
    };
}

/// # Safety
///
/// Writes exactly 1 byte to `dst`.
unsafe impl Pack for u8 {
    const LEN: usize = 1;

    #[inline(always)]
    unsafe fn pack(&self, dst: *mut u8) {
        dst.write(*self);
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
    const LEN: usize = 1;

    #[inline(always)]
    unsafe fn pack(&self, dst: *mut u8) {
        dst.write(*self as u8)
    }
}

/// # Safety
///
/// Writes exactly 32 bytes to `dst`.
unsafe impl Pack for Address {
    const LEN: usize = size_of::<Address>();

    #[inline(always)]
    unsafe fn pack(&self, dst: *mut u8) {
        core::ptr::copy_nonoverlapping(self as *const Address as *const u8, dst, Self::LEN);
    }
}

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

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use super::*;
    pub struct TestStruct {
        a: u64,
        b: u32,
        c: u8,
        d: Address,
    }

    unsafe impl Pack for TestStruct {
        const LEN: usize = 8 + 4 + 1 + 32;

        unsafe fn pack(&self, dst: *mut u8) {
            self.a.pack(dst);
            self.b.pack(dst.add(8));
            self.c.pack(dst.add(8 + 4));
        }
    }

    unsafe impl Unpack for TestStruct {
        unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError> {
            Ok(Self {
                a: u64::unpack(src)?,
                b: u32::unpack(src.add(8))?,
                c: u8::unpack(src.add(8 + 4))?,
                d: Address::unpack(src.add(8 + 4 + 1))?,
            })
        }
    }

    const TEST_STRUCT: TestStruct = TestStruct {
        a: 1024,
        b: 256,
        c: 16,
        d: Address::new_from_array([0u8; 32]),
    };

    fn expected_bytes_test_struct() -> Vec<u8> {
        [
            TEST_STRUCT.a.to_le_bytes().as_ref(),
            TEST_STRUCT.b.to_le_bytes().as_ref(),
            TEST_STRUCT.c.to_le_bytes().as_ref(),
            TEST_STRUCT.d.to_bytes().as_ref(),
        ]
        .concat()
    }

    #[test]
    fn simple_struct() {
        let res = unsafe {
            let mut packed: [MaybeUninit<u8>; TestStruct::LEN] =
                [MaybeUninit::uninit(); TestStruct::LEN];
            let packed_ptr = packed.as_mut_ptr() as *mut u8;
            TEST_STRUCT.pack(packed_ptr);

            *(packed.as_ptr() as *const [u8; TestStruct::LEN])
        };

        assert_eq!(res.as_ref(), expected_bytes_test_struct());
    }

    #[test]
    fn test_composable_pack() {
        struct StructWithStructs {
            a: u128,
            b: u32,
            c: bool,
            test_struct: TestStruct,
        }

        unsafe impl Pack for StructWithStructs {
            const LEN: usize = 16 + 4 + 1 + TestStruct::LEN;

            unsafe fn pack(&self, dst: *mut u8) {
                self.a.pack(dst);
                self.b.pack(dst.add(16));
                self.c.pack(dst.add(16 + 4));
                self.test_struct.pack(dst.add(16 + 4 + 1));
            }
        }

        unsafe impl Unpack for StructWithStructs {
            unsafe fn unpack(src: *const u8) -> Result<Self, ProgramError> {
                Ok(Self {
                    a: u128::unpack(src)?,
                    b: u32::unpack(src)?,
                    c: bool::unpack(src)?,
                    test_struct: TestStruct::unpack(src)?,
                })
            }
        }

        let complex_struct = StructWithStructs {
            a: 8192,
            b: 4096,
            c: true,
            test_struct: TEST_STRUCT,
        };

        let res = unsafe {
            let mut packed: [MaybeUninit<u8>; StructWithStructs::LEN] =
                [MaybeUninit::uninit(); StructWithStructs::LEN];
            let packed_ptr = packed.as_mut_ptr() as *mut u8;
            complex_struct.pack(packed_ptr);

            *(packed.as_ptr() as *const [u8; StructWithStructs::LEN])
        };

        assert_eq!(
            res.as_ref(),
            [
                complex_struct.a.to_le_bytes().as_ref(),
                complex_struct.b.to_le_bytes().as_ref(),
                [complex_struct.c as u8].as_ref(),
                expected_bytes_test_struct().as_ref(),
            ]
            .concat()
        );
    }
}
