#!/bin/sh
# scripts/install.sh offers rustup only after an explicit "y" in an interactive terminal.
# The npm stub exits 42 on its first real use, so the script never reaches the copy into
# /Applications.
set -u
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
fail() { echo "FAIL: $*"; exit 1; }

setup() {
  work="$(mktemp -d)"
  mkdir -p "$work/scripts" "$work/stubs" "$work/home"
  cp "$repo/scripts/install.sh" "$work/scripts/"
  # curl stub: "downloads" an installer that drops a cargo stub and ~/.cargo/env.
  cat >"$work/stubs/curl" <<'STUB'
#!/bin/sh
echo "curl $*" >>"$LOG"
cat <<'INSTALLER'
echo "rustup-init $*" >>"$LOG"
mkdir -p "$HOME/.cargo/bin"
printf '#!/bin/sh\nexit 0\n' >"$HOME/.cargo/bin/cargo"
chmod +x "$HOME/.cargo/bin/cargo"
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >"$HOME/.cargo/env"
INSTALLER
STUB
  printf '#!/bin/sh\necho "npm $*" >>"$LOG"\nexit 42\n' >"$work/stubs/npm"
  printf '#!/bin/sh\nexit 0\n' >"$work/stubs/xcode-select"
  chmod +x "$work/stubs/"*
  LOG="$work/log"; : >"$LOG"; export LOG
}

run() { # $1 answer, $2 tty flag
  (cd "$work" && printf '%s\n' "$1" | env HOME="$work/home" PATH="$work/stubs:/usr/bin:/bin" \
    SONATA_ASSUME_TTY="$2" sh scripts/install.sh >"$work/out" 2>&1)
}

setup; run n 1; code=$?
[ "$code" = 1 ] || fail "answer n should exit 1, got $code"
grep -q "rustup.rs" "$work/out" || fail "no instructions after n: $(cat "$work/out")"
grep -q curl "$LOG" && fail "curl called after n"

setup; run "" 1; code=$?
[ "$code" = 1 ] || fail "empty answer should exit 1, got $code"
grep -q curl "$LOG" && fail "curl called after empty answer"

setup; run y 1; code=$?
grep -q "curl --proto =https --tlsv1.2 -sSf https://sh.rustup.rs" "$LOG" || fail "official installer not used: $(cat "$LOG")"
grep -q "rustup-init -y" "$LOG" || fail "installer not run unattended"
[ "$code" = 42 ] || fail "script should continue to npm after installing Rust, got $code"

setup; run y 0; code=$?
[ "$code" = 1 ] || fail "no terminal should exit 1, got $code"
grep -q curl "$LOG" && fail "curl called without a terminal"
echo ok
