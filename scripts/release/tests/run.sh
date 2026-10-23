#!/bin/sh
# Runs every scripts/release/tests/*.test.sh and fails if any of them fails.
# Usage: sh scripts/release/tests/run.sh
set -u
dir="$(cd "$(dirname "$0")" && pwd)"
failed=0
for t in "$dir"/*.test.sh; do
  [ -e "$t" ] || continue
  if sh "$t" >"${TMPDIR:-/tmp}/sonata-test.out" 2>&1; then
    echo "PASS $(basename "$t")"
  else
    echo "FAIL $(basename "$t")"
    sed 's/^/  /' "${TMPDIR:-/tmp}/sonata-test.out"
    failed=1
  fi
done
exit "$failed"
