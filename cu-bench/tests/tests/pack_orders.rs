use cu_bench_tests::new_cu_bench_mollusk;
use dropset_interface::instructions::{
    BatchReplaceInstructionData,
    UnvalidatedOrders,
};
use instruction_macros_traits::Pack;
use price::{
    OrderInfoArgs,
    MANTISSA_DIGITS_LOWER_BOUND,
    MANTISSA_DIGITS_UPPER_BOUND,
};
use solana_address::Address;
use solana_instruction::Instruction;

#[test]
fn pack_orders_cu() {
    let program_id = Address::new_unique();
    let mollusk: mollusk_svm::Mollusk =
        new_cu_bench_mollusk(&program_id, "cu_bench_pack_orders.so");

    let data = BatchReplaceInstructionData::new(
        0,
        UnvalidatedOrders::new([OrderInfoArgs::new_unscaled(11_000_000, 1)]),
        UnvalidatedOrders::new([
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

#[test]
fn iterator_stops_at_first_invalid_order() {
    // Test 1: All valid orders (5)
    let all_valid = UnvalidatedOrders::new([
        OrderInfoArgs::new_unscaled(11_000_000, 1),
        OrderInfoArgs::new_unscaled(12_000_000, 2),
        OrderInfoArgs::new_unscaled(13_000_000, 3),
        OrderInfoArgs::new_unscaled(14_000_000, 4),
        OrderInfoArgs::new_unscaled(15_000_000, 5),
    ]);
    assert_eq!(all_valid.into_order_infos_iter().count(), 5);

    // Test 2: Some valid followed by invalid price_mantissa (2 valid)
    let two_valid = UnvalidatedOrders::new([
        OrderInfoArgs::new_unscaled(11_000_000, 1),
        OrderInfoArgs::new_unscaled(12_000_000, 2),
        // Invalid: price_mantissa below lower bound
        OrderInfoArgs {
            price_mantissa: MANTISSA_DIGITS_LOWER_BOUND - 1,
            base_scalar: 1,
            base_exponent_biased: 16,
            quote_exponent_biased: 16,
        },
        OrderInfoArgs::new_unscaled(14_000_000, 4),
        OrderInfoArgs::new_unscaled(15_000_000, 5),
    ]);
    assert_eq!(two_valid.into_order_infos_iter().count(), 2);

    // Test 3: First order invalid (0 valid)
    let none_valid = UnvalidatedOrders::new([
        // Invalid: price_mantissa above upper bound
        OrderInfoArgs {
            price_mantissa: MANTISSA_DIGITS_UPPER_BOUND + 1,
            base_scalar: 1,
            base_exponent_biased: 16,
            quote_exponent_biased: 16,
        },
        OrderInfoArgs::new_unscaled(12_000_000, 2),
        OrderInfoArgs::new_unscaled(13_000_000, 3),
        OrderInfoArgs::new_unscaled(14_000_000, 4),
        OrderInfoArgs::new_unscaled(15_000_000, 5),
    ]);
    assert_eq!(none_valid.into_order_infos_iter().count(), 0);

    // Test 4: One valid order
    let one_valid = UnvalidatedOrders::new([OrderInfoArgs::new_unscaled(11_000_000, 1)]);
    assert_eq!(one_valid.into_order_infos_iter().count(), 1);

    // Test 5: Zero-initialized orders (0 valid, since 0 < lower bound)
    let zero_initialized = UnvalidatedOrders::new([]);
    assert_eq!(zero_initialized.into_order_infos_iter().count(), 0);

    // Test 6: Three valid orders followed by zero-initialized
    let three_valid_then_zeros = UnvalidatedOrders::new([
        OrderInfoArgs::new_unscaled(11_000_000, 1),
        OrderInfoArgs::new_unscaled(12_000_000, 2),
        OrderInfoArgs::new_unscaled(13_000_000, 3),
    ]);
    assert_eq!(three_valid_then_zeros.into_order_infos_iter().count(), 3);

    // Test 7: Valid at boundaries
    let boundary_valid = UnvalidatedOrders::new([
        // Lower bound (valid)
        OrderInfoArgs {
            price_mantissa: MANTISSA_DIGITS_LOWER_BOUND,
            base_scalar: 1,
            base_exponent_biased: 16,
            quote_exponent_biased: 16,
        },
        // Upper bound (valid)
        OrderInfoArgs {
            price_mantissa: MANTISSA_DIGITS_UPPER_BOUND,
            base_scalar: 1,
            base_exponent_biased: 16,
            quote_exponent_biased: 16,
        },
    ]);
    assert_eq!(boundary_valid.into_order_infos_iter().count(), 2);
}
