#!/bin/sh
# Builds Sonata from source and installs it into /Applications.
# A locally built app carries no quarantine flag, so macOS opens it without a Gatekeeper prompt.
# Usage: scripts/install.sh
set -eu
cd "$(dirname "$0")/.."

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1 ($2)"; exit 1; }; }
need cargo "install Rust from https://rustup.rs"
need npm "install Node.js 22 or later"
xcode-select -p >/dev/null 2>&1 || { echo "Run: xcode-select --install"; exit 1; }

npm ci
npm run tauri build -- --bundles app

app="target/release/bundle/macos/Sonata.app"
if pgrep -x sonata >/dev/null 2>&1; then
  echo "Quit Sonata first, then run this script again."
  exit 1
fi
rm -rf /Applications/Sonata.app
cp -R "$app" /Applications/
echo "Installed /Applications/Sonata.app"
