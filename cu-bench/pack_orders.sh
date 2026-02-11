#!/bin/bash
source "$(dirname "$0")/common.sh"

run_bench "Pack/Unpack" "pack-orders" "bench-program-A" "pack_orders"
echo ""
run_bench "Borsh"       "pack-orders" "bench-program-B" "pack_orders"
