//! Test fixtures for verifying macro expansion across feature namespaces.
//!
//! This crate provides isolated environments for testing generated instruction
//! code under the two different compilation features: `client` and `program`.

#![allow(dead_code)]
#![allow(unused_imports)]

mod client;
mod events;
mod pack_and_unpack;
mod program;

use solana_address::Address;

pub const ID: Address = Address::from_str_const("TESTnXwv2eHoftsSd5NEdpH4zEu7XRC8jviuoNPdB2Q");

#[macro_export]
/// Each test fixture outputs different proc macro generated code based on the features enabled,
/// so they all must declare/create the struct.
/// This means test logic can't be consolidated with helper functions, so the best solution to avoid
/// repeating this in every file is just to output it with a macro to make it readable and DRY.
macro_rules! create_big_order_info_test {
    () => {
        const fn const_addr(array: [u8; 32]) -> Address {
            Address::new_from_array(array)
        }

        #[test]
        fn composable_pack_unpack_happy_path() {
            let info_1 = BigOrderInfo {
                deposit_1: DepositInstructionData::new(const_addr([0; 32]), 1011, true),
                bool_1: false,
                deposit_2: DepositInstructionData::new(const_addr([1; 32]), 1022, false),
                random_field: 10000,
                withdraw_1: WithdrawInstructionData::new(const_addr([2; 32]), 1033, true),
                withdraw_2: WithdrawInstructionData::new(const_addr([3; 32]), 1044, false),
                bool_2: true,
                field_2: 20000,
            };

            let info_2 = BigOrderInfo {
                deposit_1: DepositInstructionData::new(const_addr([4; 32]), 2011, false),
                bool_1: true,
                deposit_2: DepositInstructionData::new(const_addr([5; 32]), 2022, false),
                random_field: 30000,
                withdraw_1: WithdrawInstructionData::new(const_addr([6; 32]), 2033, false),
                withdraw_2: WithdrawInstructionData::new(const_addr([7; 32]), 2044, false),
                bool_2: false,
                field_2: 40000,
            };

            let info_3 = BigOrderInfo {
                deposit_1: DepositInstructionData::new(const_addr([8; 32]), 3011, true),
                bool_1: false,
                deposit_2: DepositInstructionData::new(const_addr([9; 32]), 3022, true),
                random_field: 50000,
                withdraw_1: WithdrawInstructionData::new(const_addr([10; 32]), 3033, true),
                withdraw_2: WithdrawInstructionData::new(const_addr([11; 32]), 3044, true),
                bool_2: false,
                field_2: 60000,
            };

            let big_infos =
                BigOrderInfosInstructionData::new(info_1.clone(), info_2.clone(), info_3.clone());

            let untagged = big_infos.pack();
            let tagged = big_infos.pack_tagged();

            // Assertions about length, discriminant tag byte, tagged vs untagged equality.
            assert_eq!(tagged[0], BigOrderInfosInstructionData::TAG_BYTE);
            assert_eq!(untagged.len(), BigOrderInfosInstructionData::LEN);
            assert_eq!(tagged.len(), BigOrderInfosInstructionData::LEN_WITH_TAG);
            assert_eq!(untagged, tagged[1..]);

            assert_eq!(untagged.len(), BigOrderInfo::LEN * 3);

            let unpacked = BigOrderInfosInstructionData::unpack_untagged(&untagged).unwrap();
            // Unpack and unpack_untagged should be equivalent.
            assert_eq!(
                unpacked,
                <BigOrderInfosInstructionData as Unpack>::unpack(&untagged).unwrap()
            );
            // Idempotency when packing/packing repeatedly.
            assert_eq!(unpacked.pack(), untagged);
            assert_eq!(unpacked.pack_tagged(), tagged);

            // Individual field equality checks.
            assert_eq!(untagged[0..BigOrderInfo::LEN], info_1.pack());
            assert_eq!(
                untagged[BigOrderInfo::LEN..BigOrderInfo::LEN * 2],
                info_2.pack()
            );
            assert_eq!(untagged[BigOrderInfo::LEN * 2..], info_3.pack());

            // Equality checks with PartialEq/Eq instead of just bytes.
            assert_eq!(unpacked, big_infos);
        }
    };
}
