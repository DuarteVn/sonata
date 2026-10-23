#!/bin/sh
# Runs every quality gate required before merge (see the constitution).
set -eu
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run typecheck
npm test
sh scripts/release/tests/run.sh
echo "All checks passed."
