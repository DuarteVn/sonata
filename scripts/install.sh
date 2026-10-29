#!/bin/sh
# Builds Sonata from source and installs it.
# macOS: into /Applications. A locally built app carries no quarantine flag, so macOS opens it
# without a Gatekeeper prompt.
# Linux: for the current user, under ~/.local (AppImage, sonata CLI, desktop entry, icon).
# Usage: scripts/install.sh
set -eu
cd "$(dirname "$0")/.."

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing: $1 ($2)"; exit 1; }; }
# SONATA_ASSUME_TTY=1 lets scripts/release/tests/install-prompt.test.sh answer the prompt.
interactive() { [ -t 0 ] || [ "${SONATA_ASSUME_TTY:-}" = 1 ]; }
# SONATA_UNAME and SONATA_OS_RELEASE let the tests pick the system.
system="${SONATA_UNAME:-$(uname -s)}"

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

# The system libraries Tauri needs on Linux, as one install command for the distribution family.
linux_packages() {
  ids="$(. "${SONATA_OS_RELEASE:-/etc/os-release}" 2>/dev/null && echo "${ID:-} ${ID_LIKE:-}")" || ids=""
  case " $ids " in
    *" debian "* | *" ubuntu "*)
      echo "sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libssl-dev patchelf file build-essential" ;;
    *" fedora "* | *" rhel "*)
      echo "sudo dnf install webkit2gtk4.1-devel gtk3-devel librsvg2-devel openssl-devel patchelf file gcc" ;;
    *" arch "*)
      echo "sudo pacman -S --needed webkit2gtk-4.1 gtk3 librsvg openssl patchelf file base-devel" ;;
    *)
      echo "install the WebKitGTK 4.1, GTK 3, librsvg, and OpenSSL development packages, plus patchelf and a C compiler" ;;
  esac
}

install_macos() {
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
}

install_linux() {
  if ! command -v pkg-config >/dev/null 2>&1 || ! pkg-config --exists webkit2gtk-4.1; then
    echo "Missing: WebKitGTK 4.1 development files. Install them with:"
    echo "  $(linux_packages)"
    exit 1
  fi

  npm ci
  # Update files need the release signing key; local builds skip them (005 research R7).
  npm run tauri build -- --bundles appimage --config '{"bundle":{"createUpdaterArtifacts":false}}'

  if pgrep -x sonata-app >/dev/null 2>&1; then
    echo "Quit Sonata first, then run this script again."
    exit 1
  fi
  bin="$HOME/.local/bin"
  apps="$HOME/.local/share/applications"
  icons="$HOME/.local/share/icons/hicolor/128x128/apps"
  mkdir -p "$bin" "$apps" "$icons"
  set -- target/release/bundle/appimage/*.AppImage
  [ -f "$1" ] || { echo "The build made no AppImage."; exit 1; }
  install -m 755 "$1" "$bin/sonata-app"
  install -m 755 src-tauri/resources/bin/sonata "$bin/sonata"
  install -m 644 src-tauri/icons/128x128.png "$icons/sonata.png"
  cat >"$apps/sonata.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=Sonata
Comment=Canvas for AI agent terminals, notes, and links
Exec=$bin/sonata-app
Icon=sonata
Terminal=false
Categories=Development;
EOF
  echo "Installed $bin/sonata-app, $bin/sonata, and the Sonata launcher entry"
  case ":$PATH:" in
    *":$bin:"*) ;;
    *) echo "Add $bin to your PATH to run sonata outside Sonata terminals." ;;
  esac
}

case "$system" in
  Darwin) install_macos ;;
  Linux) install_linux ;;
  *) echo "Sonata builds on macOS and Linux only (this system: $system)."; exit 1 ;;
esac
