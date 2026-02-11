use cu_bench_tests::new_cu_bench_mollusk;
use dropset_interface::instructions::{
    BatchReplaceInstructionData,
    Orders,
};
use instruction_macros_traits::Pack;
use price::OrderInfoArgs;
use solana_address::Address;
use solana_instruction::Instruction;

#[test]
fn pack_orders_cu() {
    let program_id = Address::new_unique();
    let mollusk = new_cu_bench_mollusk(&program_id, "cu_bench_pack_orders.so");

    let data = BatchReplaceInstructionData::new(
        0,
        Orders::new([OrderInfoArgs::new_unscaled(11_000_000, 1)]),
        Orders::new([
            OrderInfoArgs::new_unscaled(12_000_000, 1),
            OrderInfoArgs::new_unscaled(13_000_000, 2),
            OrderInfoArgs::new_unscaled(14_000_000, 3),
            OrderInfoArgs::new_unscaled(15_000_000, 4),
            OrderInfoArgs::new_unscaled(16_000_000, 5),
        ]),
    );

    let packed = data.pack();

    let instruction = Instruction::new_with_bytes(program_id, packed.as_ref(), vec![]);

    let result = mollusk.process_instruction(&instruction, &[]);
    assert!(
        result.program_result.is_ok(),
        "Instruction failed: {:?}",
        result.program_result
    );
    println!("Compute units consumed: {}", result.compute_units_consumed);
}
