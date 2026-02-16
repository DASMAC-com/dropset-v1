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

[1820ad9]: https://github.com/Ellipsis-Labs/phoenix-v1/commit/1820ad9208c0546be1e93b3adb534c46598e02cb
