use instruction_macros::{
    Pack,
    Unpack,
};
use solana_address::Address;

#[repr(C)]
#[derive(Pack, Unpack)]
#[cfg_attr(not(feature = "no_extra_derives"), derive(Debug, Eq, PartialEq))]
pub struct TestStruct {
    a: u8,
    b: u16,
    c: u32,
    d: u64,
    e: u128,
    f: bool,
    g: Address,
}

#[repr(C)]
#[derive(Pack, Unpack)]
#[cfg_attr(not(feature = "no_extra_derives"), derive(Debug, Eq, PartialEq))]
pub struct StructWithStructs {
    a: bool,
    b: u128,
    c: u64,
    d: u32,
    e: u16,
    f: u8,
    g: Address,
    pub test_struct: TestStruct,
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
pub(crate) mod tests {
    use std::mem::MaybeUninit;

    use super::*;

    pub const TEST_STRUCT: TestStruct = TestStruct {
        a: u8::MAX,
        b: 0,
        c: u32::MAX,
        d: 0,
        e: u128::MAX,
        f: false,
        g: Address::new_from_array([0u8; 32]),
    };

    pub const COMPLEX_TEST_STRUCT: StructWithStructs = StructWithStructs {
        a: true,
        b: 0,
        c: u64::MAX,
        d: 0,
        e: u16::MAX,
        f: 0,
        g: Address::new_from_array([2u8; 32]),
        test_struct: TEST_STRUCT,
    };

    pub fn test_struct_packed_bytes() -> Vec<u8> {
        [
            TEST_STRUCT.a.to_le_bytes().as_ref(),
            TEST_STRUCT.b.to_le_bytes().as_ref(),
            TEST_STRUCT.c.to_le_bytes().as_ref(),
            TEST_STRUCT.d.to_le_bytes().as_ref(),
            TEST_STRUCT.e.to_le_bytes().as_ref(),
            [TEST_STRUCT.f as u8].as_ref(),
            TEST_STRUCT.g.to_bytes().as_ref(),
        ]
        .concat()
    }

    pub fn complex_test_struct_packed_bytes() -> Vec<u8> {
        [
            [COMPLEX_TEST_STRUCT.a as u8].as_ref(),
            COMPLEX_TEST_STRUCT.b.to_le_bytes().as_ref(),
            COMPLEX_TEST_STRUCT.c.to_le_bytes().as_ref(),
            COMPLEX_TEST_STRUCT.d.to_le_bytes().as_ref(),
            COMPLEX_TEST_STRUCT.e.to_le_bytes().as_ref(),
            COMPLEX_TEST_STRUCT.f.to_le_bytes().as_ref(),
            COMPLEX_TEST_STRUCT.g.to_bytes().as_ref(),
            test_struct_packed_bytes().as_ref(),
        ]
        .concat()
    }

    #[test]
    fn pack_simple_struct() {
        assert_eq!(TEST_STRUCT.pack().as_ref(), test_struct_packed_bytes());
    }

    #[test]
    fn pack_complex_struct() {
        assert_eq!(
            COMPLEX_TEST_STRUCT.pack().as_ref(),
            complex_test_struct_packed_bytes()
        );
    }

    #[test]
    fn unpack_simple_struct() {
        let bytes = test_struct_packed_bytes();
        assert_eq!(TestStruct::LEN, bytes.len());
        // Safety: The length of the bytes vec was just checked as equal to the expected length.
        let res = unsafe { TestStruct::unpack(bytes.as_ptr()) };

        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TEST_STRUCT);
    }

    #[test]
    fn unpack_complex_struct() {
        let bytes = complex_test_struct_packed_bytes();
        assert_eq!(StructWithStructs::LEN, bytes.len());
        // Safety: The length of the bytes vec was just checked as equal to the expected length.
        let res = unsafe { StructWithStructs::unpack(bytes.as_ptr()) };
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), COMPLEX_TEST_STRUCT);
    }
}
