SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

run_bench() {
    local label="$1"
    local program="$2"
    local features="$3"
    local test_name="$4"

    echo "=== $label ==="
    cd "$ROOT_DIR/cu-bench/programs/$program" && cargo build-sbf --features "$features" --no-default-features 2>/dev/null
    cd "$ROOT_DIR"

    output=$(cargo test -p cu-bench-tests --test "$test_name" --quiet -- --nocapture 2>&1) || true
    if echo "$output" | grep -q "FAILED\|panicked"; then
        echo "$output"
        exit 1
    else
        echo "$output" | grep "Compute units consumed"
    fi
}
