#!/bin/sh
# scripts/release/check-version.sh compares a tag with the three declared versions.
set -u
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
fail() { echo "FAIL: $*"; exit 1; }

make_repo() { # $1 tauri version, $2 cargo version, $3 package version
  work="$(mktemp -d)"
  mkdir -p "$work/scripts/release" "$work/src-tauri"
  cp "$repo/scripts/release/check-version.sh" "$work/scripts/release/"
  printf '{\n  "productName": "Sonata",\n  "version": "%s",\n  "identifier": "dev.sonata.app"\n}\n' "$1" >"$work/src-tauri/tauri.conf.json"
  printf '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "%s"\nedition = "2024"\n\n[workspace.dependencies]\nfoo = { version = "9.9.9" }\n' "$2" >"$work/Cargo.toml"
  printf '{\n  "name": "sonata",\n  "version": "%s",\n  "dependencies": { "x": "1.0.0" }\n}\n' "$3" >"$work/package.json"
}

make_repo 0.2.0 0.2.0 0.2.0
out="$(sh "$work/scripts/release/check-version.sh" v0.2.0 2>&1)"; code=$?
[ "$code" = 0 ] || fail "all match should exit 0, got $code: $out"

make_repo 0.1.0 0.2.0 0.2.0
out="$(sh "$work/scripts/release/check-version.sh" v0.2.0 2>&1)"; code=$?
[ "$code" = 1 ] || fail "mismatch should exit 1, got $code"
echo "$out" | grep -qx "version mismatch: tag 0.2.0, src-tauri/tauri.conf.json 0.1.0" || fail "wrong message: $out"

make_repo 0.2.0 0.1.9 0.1.8
out="$(sh "$work/scripts/release/check-version.sh" v0.2.0 2>&1)"; code=$?
[ "$code" = 1 ] || fail "two mismatches should exit 1"
echo "$out" | grep -qx "version mismatch: tag 0.2.0, Cargo.toml 0.1.9" || fail "no Cargo.toml line: $out"
echo "$out" | grep -qx "version mismatch: tag 0.2.0, package.json 0.1.8" || fail "no package.json line: $out"

for bad in 0.2.0 v0.2 v0.2.0-beta x; do
  sh "$work/scripts/release/check-version.sh" "$bad" >/dev/null 2>&1; code=$?
  [ "$code" = 2 ] || fail "tag $bad should exit 2, got $code"
done
echo ok
