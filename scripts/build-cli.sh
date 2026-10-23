#!/bin/sh
# Builds the `sonata` CLI that the app ships in Contents/Resources/bin (bundle.resources).
# When Tauri builds for universal-apple-darwin (or SONATA_UNIVERSAL=1), the CLI is universal
# too, so it runs on both Apple Silicon and Intel Macs.
set -eu
cd "$(dirname "$0")/.."

out="src-tauri/resources/bin/sonata"
mkdir -p "$(dirname "$out")"
if [ "${TAURI_ENV_TARGET_TRIPLE:-}" = "universal-apple-darwin" ] || [ "${SONATA_UNIVERSAL:-}" = "1" ]; then
  cargo build -q --release -p sonata-cli --target aarch64-apple-darwin
  cargo build -q --release -p sonata-cli --target x86_64-apple-darwin
  lipo -create -output "$out" \
    target/aarch64-apple-darwin/release/sonata \
    target/x86_64-apple-darwin/release/sonata
else
  cargo build -q --release -p sonata-cli
  cp target/release/sonata "$out"
fi
chmod 755 "$out"
