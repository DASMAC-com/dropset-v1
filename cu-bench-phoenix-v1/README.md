# Description

You can run the `phoenix-v1` CU benchmark tests with either of the following
commands:

```shell
pnpm run bench:phoenix
bash run-bench.sh
```

If you want to run the `cargo test` command yourself, you must ensure the
`SBF_OUT_DIR` environment variable is set to the directory where the
`phoenix.so` is located. If `SBF_OUT_DIR` is not set, CUs won't be properly
measured and will appear to be extraordinarily low.

## `phoenix` Program version

These benchmarks use the `phoenix.so` program deployed on `mainnet-beta` as of
February 16, 2026. The `master` branch for the `phoenix-v1` program as of
that same date is at commit [1820ad9]. This commit is the `rev` specified
in the `phoenix-v1` `Cargo.toml` dependency, used in the test helper functions.

You can also dump the current program deployed on mainnet yourself:

```shell
solana program dump PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY \
  phoenix.so --url https://api.mainnet-beta.solana.com
```

Ensure the dumped `phoenix.so` file is in the directory specified in your
shell's `SBF_OUT_DIR` env var when running the tests.

## Test structure

Each test measures the CU consumed by a single program instruction using
`solana-program-test`. The instruction is simulated to capture `units_consumed`,
then processed to apply state changes.

**Single-instruction tests** (measured once):

- **Deposit** — deposit tokens into the market.
- **Withdraw** — withdraw tokens from the market.
- **PlaceLimitOrder** — place a single post-only limit order.

**Batched tests** (measured at 1, 10, and 50 items per instruction):

- **PlaceMultiplePostOnly** — place N orders in a single `MultipleOrderPacket`
  instruction. Total CU is divided by N to get the amortized per-order cost.
- **CancelAllOrders** — cancel N resting orders with a single
  `CancelAllOrdersWithFreeFunds` instruction. Total CU is divided by N.
- **1 Swap, N Fills** — place N resting asks, then send a single IOC buy order
  that fills against all N of them. This is one swap instruction that matches
  N times, not N separate swaps. Total CU is divided by N.
- **MultipleOrderPacket (Batch)** — place N orders then cancel all N, measuring
  each operation separately within the same test.

## Limitations

These benchmarks run against a fresh, empty order book with a single trader.
Real-world order books on mainnet will have more resting orders, more traders,
and different state layouts, all of which can affect CU consumption.

The tests do pass known trader seat information where possible, so these are
not worst-case measurements — they reflect the CU costs a reasonably optimized
client would see.

These results should be treated as a baseline, not a definitive measure of
production CU costs. Cross-referencing with actual mainnet transaction data is
recommended for a more complete picture.

[1820ad9]: https://github.com/Ellipsis-Labs/phoenix-v1/commit/1820ad9208c0546be1e93b3adb534c46598e02cb
