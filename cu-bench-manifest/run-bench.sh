#!/usr/bin/env bash
set -euo pipefail

TAG="program-v3.0.10"
REPO="Bonasa-Tech/manifest"

ROOT="$(git rev-parse --show-toplevel)"
DIR="$ROOT/cu-bench-manifest/.cache/sbf/manifest/$TAG"
mkdir -p "$DIR"

BASE="https://github.com/$REPO/releases/download/$TAG"

# Download only if missing; keep curl quiet unless there's an error.
for f in manifest.so wrapper.so ui_wrapper.so; do
  if [[ -s "$DIR/$f" ]]; then
    continue
  fi

  curl -fsSL --retry 3 --retry-delay 1 -o "$DIR/$f" "$BASE/$f"
done

export RUST_LOG='solana_program_test=warn,solana_runtime::message_processor::stable_log=info,solana_rbpf::vm=info'
export SBF_OUT_DIR="$DIR"

cd "$ROOT/cu-bench-manifest"
cargo test --quiet -p cu-bench-manifest -- --nocapture --test-threads=1 --format=terse 2>&1
