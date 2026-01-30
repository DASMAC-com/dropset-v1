use instruction_macros::Pack;
use solana_address::Address;

#[derive(Pack)]
pub struct TestStruct {
    a: u64,
    b: u32,
    c: u8,
    d: Address,
}
