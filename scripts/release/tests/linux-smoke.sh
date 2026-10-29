#!/bin/sh
# Starts the Linux AppImage on a virtual display and checks that Sonata comes up (feature 011).
# Runs on the linux-app CI job; this Mac cannot run Linux.
# Usage: sh scripts/release/tests/linux-smoke.sh <Sonata.AppImage> <sonata.deb> <screenshot.png>
set -eu
appimage="$1"
deb="$2"
shot="$3"

fail() { echo "FAIL: $*"; exit 1; }

home="$(mktemp -d /tmp/sonata-smoke.XXXXXX)"
log="$home/app.log"
chmod +x "$appimage"
# A short HOME keeps the socket path under the Unix socket length limit.
HOME="$home" xvfb-run -a -s "-screen 0 1400x900x24" sh -c '
  "$1" >"$2" 2>&1 &
  app=$!
  i=0
  while [ ! -S "$HOME/.sonata/run/sonata.sock" ]; do
    i=$((i + 1))
    [ "$i" -le 120 ] || { kill "$app"; exit 3; }
    kill -0 "$app" 2>/dev/null || exit 4
    sleep 0.5
  done
  echo "socket ready"
  sleep 5
  import -window root "$3"
  kill "$app"
  wait "$app" 2>/dev/null || true
' smoke "$(realpath "$appimage")" "$log" "$(realpath -m "$shot")" || {
  code=$?
  sed 's/^/  app: /' "$log" || true
  case "$code" in
    3) fail "no IPC socket after 60 s" ;;
    4) fail "the app exited before its IPC socket appeared" ;;
    *) fail "smoke run failed ($code)" ;;
  esac
}

cli="$home/.local/bin/sonata"
[ -x "$cli" ] || fail "no CLI copy at ~/.local/bin/sonata"
"$cli" help >/dev/null || fail "the CLI copy does not run"
echo "cli copied"

listing="$(dpkg-deb -c "$deb")"
echo "$listing" | grep -Eq ' (\./)?usr/bin/sonata$' || {
  echo "$listing" | sed 's/^/  deb: /'
  fail "the .deb has no /usr/bin/sonata"
}
echo "deb has /usr/bin/sonata"
rm -rf "$home"
