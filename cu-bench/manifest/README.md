# Description

You can run the `manifest` CU benchmark tests with either of the following
commands:

```shell
pnpm run bench:manifest
bash run-bench.sh
```

If you want to run the `cargo test` command yourself, you must ensure the
`SBF_OUT_DIR` environment variable is set to the directory where the
`manifest.so` is located. If `SBF_OUT_DIR` is not set, CUs won't be properly
measured and will appear to be extraordinarily low.

## `manifest` Program version

These benchmarks use the `manifest.so` program deployed on `mainnet-beta` as of
February 16, 2026. The `manifest` program as of that same date is at tag
[program-v3.0.10]. This tag is the `tag` specified in the `manifest-dex`
`Cargo.toml` dependency, used in the test helper functions.

You can also dump the current program deployed on mainnet yourself:

```shell
solana program dump MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms \
  manifest.so --url https://api.mainnet-beta.solana.com
```

Ensure the dumped `manifest.so` file is in the directory specified in your
shell's `SBF_OUT_DIR` env var when running the tests.

## Test structure

Each test measures the CU consumed by a single program instruction using
`solana-program-test`. The instruction is simulated to capture `units_consumed`,
then processed to apply state changes.

**Single-instruction tests** (measured once):

- **Deposit** — deposit tokens into the market.
- **Withdraw** — withdraw tokens from the market.

**Batched tests** (measured at 1, 10, and 50 items per instruction):

- **BatchUpdate (Place)** — place N orders in a single `batch_update`
  instruction. Total CU is divided by N to get the amortized per-order cost.
- **BatchUpdate (Cancel)** — cancel N resting orders with a single
  `batch_update` instruction. Total CU is divided by N.
- **Swap** — place N resting asks, then send a single swap instruction sized to
  fill against all N of them. This is one swap that matches N times, not N
  separate swaps. Total CU is divided by N.

Each batched test is run twice: once on a fresh market, and once on a
pre-expanded market (using `expand_market` to pre-allocate order book space).
The pre-expanded variant isolates the cost of the operation itself from the
cost of allocating new space in the order book.

## Limitations

These benchmarks run against a fresh, empty order book with a single trader.
Real-world order books on mainnet will have more resting orders, more traders,
and different state layouts, all of which can affect CU consumption.

The tests do pass known trader index and order data index hints where possible,
so these are not worst-case measurements — they reflect the CU costs a
reasonably optimized client would see.

These results should be treated as a baseline, not a definitive measure of
production CU costs. Cross-referencing with actual mainnet transaction data is
recommended for a more complete picture.

[program-v3.0.10]: https://github.com/Bonasa-Tech/manifest/releases/tag/program-v3.0.10
