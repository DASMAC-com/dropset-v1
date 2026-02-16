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

[program-v3.0.10]: https://github.com/Bonasa-Tech/manifest/releases/tag/program-v3.0.10
