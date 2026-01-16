#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./poll_oanda_candles.sh [per_minute] [instrument] [granularity]
#
# Examples:
#   ./poll_oanda_candles.sh               # defaults: 110 EUR_USD S5
#   ./poll_oanda_candles.sh 90            # 90/min
#   ./poll_oanda_candles.sh 60 GBP_USD S10
#
# Requires:
#   - OANDA_AUTH env var set (Bearer token)
#   - jq installed

PER_MINUTE="${1:-110}"
INSTRUMENT="${2:-EUR_USD}"
GRANULARITY="${3:-S5}"

if [[ -z "${OANDA_AUTH:-}" ]]; then
  echo "Error: OANDA_AUTH is not set. Export your token into OANDA_AUTH." >&2
  exit 1
fi

# Basic validation
if ! [[ "$PER_MINUTE" =~ ^[0-9]+$ ]] || (( PER_MINUTE < 1 || PER_MINUTE > 120 )); then
  echo "Error: per_minute must be an integer between 1 and 120." >&2
  exit 1
fi

# Compute sleep interval in seconds (float). Example: 110/min => ~0.54545s
SLEEP_SECS="$(awk -v n="$PER_MINUTE" 'BEGIN { printf "%.6f\n", 60.0 / n }')"

URL="https://api-fxpractice.oanda.com/v3/instruments/${INSTRUMENT}/candles?count=1&granularity=${GRANULARITY}"

echo "Polling ${INSTRUMENT} candles (${GRANULARITY}) at ${PER_MINUTE}/min (sleep ${SLEEP_SECS}s)"
echo "Ctrl+C to stop."
echo

while true; do
  # Print the whole JSON via jq -r .
  curl -sS -H "Authorization: Bearer ${OANDA_AUTH}" "$URL" | jq -r .
  sleep "$SLEEP_SECS"
done

