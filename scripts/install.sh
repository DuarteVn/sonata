#!/bin/sh
# Builds Sonata from source and installs it into /Applications.
# A locally built app carries no quarantine flag, so macOS opens it without a Gatekeeper prompt.
# Usage: scripts/install.sh
set -eu
cd "$(dirname "$0")/.."

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1 ($2)"; exit 1; }; }
# SONATA_ASSUME_TTY=1 lets scripts/release/tests/install-prompt.test.sh answer the prompt.
interactive() { [ -t 0 ] || [ "${SONATA_ASSUME_TTY:-}" = 1 ]; }

if ! command -v cargo >/dev/null 2>&1; then
  answer=""
  if interactive; then
    printf "Rust is not installed. Install it with rustup (https://rustup.rs)? [y/N] "
    read -r answer || answer=""
  fi
  case "$answer" in
    y | Y | yes | YES)
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      . "$HOME/.cargo/env"
      ;;
    *)
      echo "Missing: cargo (install Rust from https://rustup.rs, then run this script again)"
      exit 1
      ;;
  esac
fi
need npm "install Node.js 22 or later"
xcode-select -p >/dev/null 2>&1 || { echo "Run: xcode-select --install"; exit 1; }

npm ci
# Update archives need the release signing key; local builds skip them (005 research R7).
npm run tauri build -- --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'

app="target/release/bundle/macos/Sonata.app"
if pgrep -x sonata-app >/dev/null 2>&1; then
  echo "Quit Sonata first, then run this script again."
  exit 1
fi
rm -rf /Applications/Sonata.app
cp -R "$app" /Applications/
echo "Installed /Applications/Sonata.app"
