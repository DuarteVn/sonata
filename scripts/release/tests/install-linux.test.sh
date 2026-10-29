#!/bin/sh
# scripts/install.sh on Linux: names the packages to install when WebKitGTK is missing, builds
# the AppImage without update files, and installs for the current user under ~/.local
# (feature 011). Tools are stubbed, so nothing is built or installed for real.
set -u
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
fail() { echo "FAIL: $*"; exit 1; }

setup() { # $1: 0 when pkg-config finds WebKitGTK
  work="$(mktemp -d)"
  mkdir -p "$work/scripts" "$work/stubs" "$work/home" "$work/src-tauri/resources/bin" \
    "$work/src-tauri/icons"
  cp "$repo/scripts/install.sh" "$work/scripts/"
  printf 'cli' >"$work/src-tauri/resources/bin/sonata"
  printf 'png' >"$work/src-tauri/icons/128x128.png"
  LOG="$work/log"; : >"$LOG"; export LOG
  printf '#!/bin/sh\nexit 0\n' >"$work/stubs/cargo"
  printf '#!/bin/sh\nexit %s\n' "$1" >"$work/stubs/pkg-config"
  printf '#!/bin/sh\nexit 1\n' >"$work/stubs/pgrep"
  # npm stub: records its arguments; the build "makes" an AppImage.
  cat >"$work/stubs/npm" <<'STUB'
#!/bin/sh
echo "npm $*" >>"$LOG"
case "$*" in *"tauri build"*)
  mkdir -p target/release/bundle/appimage
  printf 'appimage' >target/release/bundle/appimage/Sonata_0.0.0_amd64.AppImage ;;
esac
STUB
  chmod +x "$work/stubs/"*
}

run() { # $1: os-release content
  printf '%s\n' "$1" >"$work/os-release"
  (cd "$work" && env HOME="$work/home" PATH="$work/stubs:/usr/bin:/bin" SONATA_UNAME=Linux \
    SONATA_OS_RELEASE="$work/os-release" sh scripts/install.sh >"$work/out" 2>&1)
}

setup 1; run 'ID=ubuntu
ID_LIKE=debian'; code=$?
[ "$code" = 1 ] || fail "missing WebKitGTK should exit 1, got $code"
grep -q "sudo apt install libwebkit2gtk-4.1-dev" "$work/out" || fail "no apt command: $(cat "$work/out")"
grep -q "npm" "$LOG" && fail "built without WebKitGTK"

setup 1; run 'ID=fedora'; code=$?
[ "$code" = 1 ] || fail "fedora: expected exit 1, got $code"
grep -q "sudo dnf install webkit2gtk4.1-devel" "$work/out" || fail "no dnf command: $(cat "$work/out")"

setup 1; run 'ID=arch'; code=$?
grep -q "sudo pacman -S --needed webkit2gtk-4.1" "$work/out" || fail "no pacman command: $(cat "$work/out")"

setup 0; run 'ID=ubuntu'; code=$?
[ "$code" = 0 ] || fail "install should succeed, got $code: $(cat "$work/out")"
grep -q -- "tauri build -- --bundles appimage --config" "$LOG" || fail "no AppImage build: $(cat "$LOG")"
grep -q 'createUpdaterArtifacts":false' "$LOG" || fail "update files not skipped"
h="$work/home/.local"
[ -x "$h/bin/sonata-app" ] || fail "no ~/.local/bin/sonata-app"
[ "$(cat "$h/bin/sonata")" = cli ] || fail "no ~/.local/bin/sonata"
[ -f "$h/share/icons/hicolor/128x128/apps/sonata.png" ] || fail "no icon"
grep -q "^Exec=$h/bin/sonata-app$" "$h/share/applications/sonata.desktop" || fail "bad desktop entry"
echo ok
