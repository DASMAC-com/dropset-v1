use client::e2e_helpers::no_bias_order_info_args;
use cu_bench_tests::new_cu_bench_mollusk;
use dropset_interface::instructions::{
    BatchReplaceInstructionData,
    Orders,
};
use instruction_macros_traits::Pack;
use solana_instruction::Instruction;

#[test]
fn pack_orders_cu() {
    let mollusk = new_cu_bench_mollusk(&cu_bench_pack_orders::ID, "cu_bench_pack_orders.so");

    let data = BatchReplaceInstructionData::new(
        0,
        Orders::new([no_bias_order_info_args(11_000_000, 1)]),
        Orders::new([
            no_bias_order_info_args(12_000_000, 1),
            no_bias_order_info_args(13_000_000, 2),
            no_bias_order_info_args(14_000_000, 3),
            no_bias_order_info_args(15_000_000, 4),
            no_bias_order_info_args(16_000_000, 5),
        ]),
    );

    let packed = data.pack();

    let instruction =
        Instruction::new_with_bytes(cu_bench_pack_orders::ID, packed.as_ref(), vec![]);

    let result = mollusk.process_instruction(&instruction, &[]);
    assert!(
        result.program_result.is_ok(),
        "Instruction failed: {:?}",
        result.program_result
    );
    println!("Compute units consumed: {}", result.compute_units_consumed);
}
