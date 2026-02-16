#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"

export RUST_LOG="solana_program_test=warn,solana_runtime::message_processor::stable_log=info,solana_rbpf::vm=info"
export SBF_OUT_DIR="$ROOT/cu-bench/manifest"

cd "$ROOT/cu-bench/manifest"
cargo test --quiet -p cu-bench-manifest -- --nocapture --test-threads=1 --format=terse 2>&1
