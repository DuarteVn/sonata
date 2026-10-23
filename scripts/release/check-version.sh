#!/bin/sh
# Checks that a release tag matches the version declared in tauri.conf.json, Cargo.toml
# ([workspace.package]), and package.json.
# Usage: scripts/release/check-version.sh vMAJOR.MINOR.PATCH
# Exit: 0 all match, 1 mismatch (one line per file), 2 malformed tag.
set -u
cd "$(dirname "$0")/../.."

tag="${1:-}"
if ! echo "$tag" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+$'; then
  echo "tag must look like v1.2.3, got '$tag'" >&2
  exit 2
fi
want="${tag#v}"

first_version() { # first "version": "x" in a JSON file
  sed -n 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -n 1
}
cargo_version() { # version = "x" inside [workspace.package]
  awk '/^\[/{in_pkg = ($0 == "[workspace.package]")} in_pkg && /^version[[:space:]]*=/{gsub(/.*=[[:space:]]*"|".*/, ""); print; exit}' Cargo.toml
}

status=0
check() { # $1 file, $2 found version
  if [ "$2" != "$want" ]; then
    echo "version mismatch: tag $want, $1 ${2:-<missing>}"
    status=1
  fi
}
check src-tauri/tauri.conf.json "$(first_version src-tauri/tauri.conf.json)"
check Cargo.toml "$(cargo_version)"
check package.json "$(first_version package.json)"
exit "$status"
