# cu-bench-manifest

Local compute-unit (CU) benchmarks for the [Manifest](https://github.com/CKS-Systems/manifest) on-chain orderbook (`MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms`).

The crate measures exact CU consumption of individual Manifest instructions using `solana-program-test` (BanksClient). Because BanksClient runs the SBF program in a deterministic VM, results are perfectly reproducible — the same program binary + the same operations always yield the same CU count.

## Quick start

```bash
pnpm run cu-bench-manifest
```

This does three things (see [run-bench.sh](run-bench.sh)):

1. Downloads the pre-built `manifest.so`, `wrapper.so`, and `ui_wrapper.so` from the Manifest GitHub release (`program-v3.0.10`) into `.cache/sbf/manifest/` (skipped if already cached).
2. Points `SBF_OUT_DIR` at the cached `.so` files so `solana-program-test` picks them up.
3. Runs `cargo test` with `--nocapture` so CU numbers print to stdout.

Tests are compiled with release-like settings (`opt-level = 3`, `lto = "fat"`, `codegen-units = 1`) to match on-chain program behaviour.

## What it measures

Every test in [tests/cu_measurement.rs](tests/cu_measurement.rs) follows the same pattern:

1. **Spin up a local validator** via `ProgramTest` with the Manifest program loaded.
2. **Warm up a market** — claim a seat, deposit both tokens, pre-expand the market (32 free blocks), and place 10 seed orders (5 bids + 5 asks) so the book is non-empty.
3. **Execute a single instruction** and read `metadata.compute_units_consumed` from the `BanksClient` result.

The warmup ensures benchmarks hit the typical hot path: no first-time account expansion, non-empty orderbook, pre-existing seat.

### Operations benchmarked

| Test | What it does |
|------|-------------|
| `cu_deposit` | Deposit SOL into the market |
| `cu_batch_update_place_1` | Place 1 limit ask |
| `cu_batch_update_cancel_1` | Cancel 1 resting order |
| `cu_batch_update_cancel_1_place_1` | Cancel 1 + place 1 (order replacement) |
| `cu_batch_update_cancel_4_place_4` | Cancel 4 + place 4 (bulk replacement) |
| `cu_swap_fill_1` | Swap that fills 1 resting order |
| `cu_swap_fill_3` | Swap that fills 3 resting orders |
| `cu_withdraw` | Withdraw SOL from the market |
| `measure_many_maker_batch_replace` | 5 rounds of cancel-4 + place-4 (maker spam) |

### Example output

```
BatchUpdate (cancel 1)             2668 CU
BatchUpdate (cancel 1 + place 1)   3888 CU
BatchUpdate (cancel 4 + place 4)  10485 CU
BatchUpdate (place 1)              3107 CU
Deposit                            9676 CU
Swap (fill 1 order)               18367 CU
Swap (fill 3 orders)              18367 CU
Withdraw                          10092 CU

Maker spam: cancel 4 + place 4, 5 times
  Round 1   10485 CU
  Round 2   10486 CU
  Round 3   10483 CU
  Round 4   10486 CU
  Round 5   10482 CU
  Average   10484 CU
```

## How CU measurement works

The key function is `send_tx_measure_cu` in [src/utils.rs](src/utils.rs). It sends a transaction through `BanksClient::process_transaction_with_metadata` and reads back the CU consumed:

```rust
let result = context.banks_client
    .process_transaction_with_metadata(tx)
    .await
    .unwrap();
let metadata = result.metadata.expect("metadata should be present");
metadata.compute_units_consumed
```

This is the same mechanism Manifest uses in their own CI benchmarks ([example run](https://github.com/Bonasa-Tech/manifest/actions/runs/21839071570)).

## Project structure

```
cu-bench-manifest/
├── run-bench.sh                  # Download .so files + run tests
├── Cargo.toml                    # Standalone workspace, pinned to manifest program-v3.0.10
├── src/
│   ├── lib.rs                    # Re-exports fixtures + utils
│   ├── utils.rs                  # send_tx_measure_cu, warm_up_market, batch_update_ix, etc.
│   └── fixtures/
│       ├── test_fixture.rs       # TestFixture: spins up ProgramTest with mints, market, globals
│       ├── market_fixture.rs     # MarketFixture: create + reload a Manifest market
│       ├── mint_fixture.rs       # MintFixture: create SPL mints
│       ├── token_account_fixture.rs
│       └── global_fixture.rs
└── tests/
    └── cu_measurement.rs         # The actual benchmark tests
```

## Example mainnet transactions

Real Manifest transactions on Solana mainnet for reference:

- [4h9iqxS...K6kv](https://explorer.solana.com/tx/4h9iqxSKUqjBTayascN1DC7Qu4agi9M24CLiQyu4DffwEFj5re3K2a5kVu36PTtyK1Uva45rKqJ9j5LQDNnLK6kv)
- [2TuabWZ...zYSL](https://explorer.solana.com/tx/2TuabWZ1jys4VKdxcw6VYojMbjSYGDN2ufqqTUkEpLKjEVbuRF8RW9VQ9qzYyQMuXxKfJBeHEvtXDwYsLd6fzYSL)
- [4guXFdc...7tXY](https://explorer.solana.com/tx/4guXFdcVRVXd7JCTfjS5ZcWGj5NkswfBZWfCRAJMJiUSmWhBQdnUehNuVoD8Z41bz4wMFvnCi2L5LjhdCjGT7tXY)
- [4Rnc96x...NDzi](https://explorer.solana.com/tx/4Rnc96x7smHPokdWZeJVNEw7oP3TWBDye5ZqjseQ4tr6WKN1yyxtTYf2K1zkGVxCCqbgVw79MDpZ2w7MvVGKNDzi)
- [hwxmyX4...bgy](https://explorer.solana.com/tx/hwxmyX4aZUzoY9HYgvZeCSknNJkk3wwGSSjG36KmSNHUGbGmpVW8UA7XBR5iBowr7CMnrENUJLrUvdoQ8bfGbgy)