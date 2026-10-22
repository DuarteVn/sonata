#!/bin/sh
# Records the raw terminal output of a command into a fixture file for screen-model tests.
# Usage: scripts/capture-pty.sh <out-file> <seconds> <command...>
# The command runs in a 100x30 pseudo-terminal (via `script`) and is killed after <seconds>.
set -eu
out=$1; secs=$2; shift 2
tmp=$(mktemp)
script -q "$tmp" sh -c 'stty cols 100 rows 30; exec "$@"' sh "$@" </dev/null >/dev/null 2>&1 &
pid=$!
sleep "$secs"
pkill -TERM -P "$pid" 2>/dev/null || true
sleep 0.5
pkill -KILL -P "$pid" 2>/dev/null || true
kill "$pid" 2>/dev/null || true
wait "$pid" 2>/dev/null || true
mv "$tmp" "$out"
echo "captured $(wc -c < "$out") bytes into $out"
