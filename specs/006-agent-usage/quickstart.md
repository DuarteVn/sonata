# Quickstart: Agent Usage Meter

Validation guide for feature 006. Contracts: [contracts/ipc.md](contracts/ipc.md),
[contracts/usage-provider.md](contracts/usage-provider.md). Model:
[data-model.md](data-model.md).

## Prerequisites

- macOS 15+, Rust stable, Node 22, repo dependencies installed (`npm ci`).
- For live checks: Claude Code installed and logged in with a subscription plan. Codex optional.
- Throwaway data folder for every run:

  ```sh
  export SONATA_HOME="$(mktemp -d)/sonata"
  ```

## Automated checks

```sh
sh scripts/check.sh            # cargo test, clippy -D warnings, fmt --check, tsc, vitest
cargo test -p sonata-core usage   # usage module only
```

Expected: all pass. Core tests use recorded fixtures and fake `claude`/`codex` executables on a
temporary `PATH`; no test contacts a provider.

## Manual scenarios

Run `npm run tauri dev` with the `SONATA_HOME` above.

1. **Consent (FR-012)**: at start, a dialog asks to read Claude Code usage and shows the exact
   probe command and its cost. Choose "Don't allow": the Usage button shows no Claude values,
   Settings → Usage shows Claude switched off. Turn the switch on: the dialog appears again;
   allow it.
2. **Panel (US1)**: click the Usage button. Claude Code shows plan (for example "Team"), account,
   5-hour and weekly percentages with reset times. In a Claude terminal run `/usage` and compare:
   percentages within 1 point, reset times within 1 minute (SC-002).
3. **Refresh (US1-2)**: press refresh; values update in under 5 s and the reading time changes.
4. **Stale (US1-5)**: turn Wi-Fi off, wait 16 minutes (or set the clock in a test build), open
   the panel: values show as stale with the last good time; turn Wi-Fi on.
5. **Not logged in (US1-3)**: make the fake provider of scenario 9 print
   `{"status":"notLoggedIn","message":"Run fake login."}`; the panel shows "Not logged in" with
   that hint and no percentages.
6. **Warning (US2)**: add the custom provider from scenario 9 printing `usedPercent: 85`; press
   "Refresh all": one notification "… 85% …"; refresh again: no second notification. Change it to
   100: one more notification and the meter shows the full state. Change `resetsAt` to a new
   time and print 85 again: a new notification (new period).
7. **Warnings off (US2-4)**: Settings → Usage → turn warnings off, repeat scenario 6 with a new
   `resetsAt`: no notification, meter still changes.
8. **Master switch (US4-1)**: turn usage off; the meter shows the off state; `ps` shows no
   `claude -p` probe for 6 minutes.
9. **Custom provider (US5)**: Settings → Usage → "Copy path", then:

   ```sh
   DIR="$SONATA_HOME/usage-providers"
   BIN="$(mktemp -d)"
   cat > "$BIN/fake.sh" <<'EOF'
   #!/bin/sh
   echo '{"plan":"Test","windows":[{"kind":"short","usedPercent":42,"resetsAt":"2030-01-01T00:00:00Z"}]}'
   EOF
   chmod +x "$BIN/fake.sh"
   printf '{"type":"usageProvider","schemaVersion":1,"payload":{"id":"fake","name":"Fake","command":["%s"]}}\n' "$BIN/fake.sh" > "$DIR/fake.json"
   ```

   Press "Refresh all": a consent dialog shows the exact path; allow; a "Fake" row and panel
   section show 42%. Edit `fake.json` to add an argument: consent is asked again.
10. **Invalid definition (US5-2)**: write `{"type":"usageProvider"}` to `bad.json`: its row
    shows the error; Claude and Fake still read.
11. **Timeout and sandbox (FR-019)**: make `fake.sh` run `sleep 30`: after 10 s the row shows
    "timed out after 10 s" and no `sleep` process remains. Make it run
    `ls ~/Library/Keychains`: the row shows unavailable (non-zero exit).
12. **Settings window (FR-015)**: Cmd+, opens Settings; Cmd+, again focuses the same window.
    Change Claude's color: the panel and meter use it.

## No credential check (SC-005)

After the session:

```sh
grep -rIl -E 'sk-ant-|oauth|access_?token|refresh_?token' "$SONATA_HOME" || echo "clean"
grep -rI '@' "$SONATA_HOME/usage-cache.json" || echo "no email in cache"
```

Expected: `clean` and `no email in cache`. Also check the log file under `$SONATA_HOME/logs/`.
