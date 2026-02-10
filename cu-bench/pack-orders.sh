#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Pack/Unpack ==="
cd "$ROOT_DIR/cu-bench/programs/pack-orders" && cargo build-sbf --features pack --no-default-features 2>/dev/null
cd "$ROOT_DIR"
cargo test -p cu-bench-tests --test pack_orders -- --nocapture 2>&1 | grep "Compute units consumed"

echo ""
echo "=== Borsh ==="
cd "$ROOT_DIR/cu-bench/programs/pack-orders" && cargo build-sbf --features borsh --no-default-features 2>/dev/null
cd "$ROOT_DIR"
cargo test -p cu-bench-tests --test pack_orders -- --nocapture 2>&1 | grep "Compute units consumed"
