#!/bin/sh
# scripts/build-cli.sh builds a host CLI by default and a universal one for universal builds.
set -eu
repo="$(cd "$(dirname "$0")/../../.." && pwd)"

setup() {
  work="$(mktemp -d)"
  mkdir -p "$work/scripts" "$work/stubs"
  cp "$repo/scripts/build-cli.sh" "$work/scripts/"
  # cargo stub: logs its arguments and creates the binary it would build.
  cat >"$work/stubs/cargo" <<'STUB'
#!/bin/sh
echo "cargo $*" >>"$LOG"
dir=target/release
case "$*" in *"--target "*) dir="target/$(echo "$*" | sed 's/.*--target \([^ ]*\).*/\1/')/release" ;; esac
mkdir -p "$dir" && echo bin >"$dir/sonata"
STUB
  cat >"$work/stubs/lipo" <<'STUB'
#!/bin/sh
echo "lipo $*" >>"$LOG"
while [ $# -gt 0 ]; do [ "$1" = "-output" ] && { echo universal >"$2"; }; shift; done
STUB
  chmod +x "$work/stubs/cargo" "$work/stubs/lipo"
  LOG="$work/log"; export LOG
}

fail() { echo "FAIL: $*"; cat "$LOG" 2>/dev/null; exit 1; }

# Host build.
setup
(cd "$work" && env -u TAURI_ENV_TARGET_TRIPLE -u SONATA_UNIVERSAL PATH="$work/stubs:$PATH" sh scripts/build-cli.sh)
grep -q "cargo build -q --release -p sonata-cli$" "$LOG" || fail "host build not run"
grep -q lipo "$LOG" && fail "lipo used for host build"
[ -x "$work/src-tauri/resources/bin/sonata" ] || fail "host CLI not installed"

# Universal build from Tauri's target triple and from the fallback variable.
for mode in TAURI_ENV_TARGET_TRIPLE=universal-apple-darwin SONATA_UNIVERSAL=1; do
  setup
  (cd "$work" && env -u TAURI_ENV_TARGET_TRIPLE -u SONATA_UNIVERSAL "$mode" PATH="$work/stubs:$PATH" sh scripts/build-cli.sh)
  grep -q -- "--target aarch64-apple-darwin" "$LOG" || fail "$mode: no arm64 build"
  grep -q -- "--target x86_64-apple-darwin" "$LOG" || fail "$mode: no x86_64 build"
  grep -q "lipo -create" "$LOG" || fail "$mode: no lipo"
  [ "$(cat "$work/src-tauri/resources/bin/sonata")" = universal ] || fail "$mode: CLI not universal"
  [ -x "$work/src-tauri/resources/bin/sonata" ] || fail "$mode: CLI not executable"
done
echo ok
