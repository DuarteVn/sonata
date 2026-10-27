# Quickstart: Kimi Code CLI as a Supported Agent

How to check that feature 010 works. Automated checks run without Kimi; the live checks need
Kimi Code CLI 2.3+ (`npm i -g @moonshot-ai/kimi-code`, then `kimi login`).

## Automated

```sh
cargo test -p sonata-core --test kimi_config   # config edit, status, backups, kimi doctor, App
cargo test -p sonata-core --test kimi_hooks    # agent file, wrapper flags, recruit, `_hook kimi …`
cargo test -p sonata-core --test kimi_session  # fake `kimi` in terminals: instructions, how asks end
npm test -- kimi                           # notice, Settings section, preset
scripts/check.sh                           # fmt, clippy, all tests, type check
```

The fake `kimi` (a script in a temporary `SONATA_AGENT_BIN_DIR`) records its arguments and, when
asked, exits 1 with `Session "<id>" not found.`. Config tests use fixture files in a temporary
`KIMI_CODE_HOME`, never `~/.kimi-code`. See [contracts/cli.md](contracts/cli.md) and
[contracts/kimi-config.md](contracts/kimi-config.md).

## Live checks (Kimi installed)

1. **Preset and instructions (US1, no setup)**: New terminal → Kimi Code, role "Reviewer". Ask
   "what can you do in Sonata, and what is your role?". Expected: Sonata commands and "Reviewer".
   From a linked Claude Code terminal, `sonata ask <kimi> "say hi"`: answer comes back after
   ~3 s of quiet.
2. **Notice**: the Kimi node shows the "tracking and restore are off" line. Dismiss → hidden; it
   stays hidden after reopening Sonata.
3. **Setup (US2)**: Settings › Agents › Kimi Code → Apply → confirm. Check:
   - `config.toml` has the block from contracts/kimi-config.md at the end; `diff` against the
     backup shows only added lines.
   - `kimi doctor` passes. Record its exit code on success and on a broken file (research R4).
   - In Terminal.app (outside Sonata), run `kimi`, send a prompt: no hook output, no delay.
4. **Busy/done (US3)**: restart the Kimi terminal. Ask it to `sleep 25 && echo done` and report.
   The asking agent gets the full answer after Kimi ends its turn, not after 3 s. Press Esc during
   a turn: the terminal stops showing busy (`Interrupt`).
5. **Restore (US4, needs 009)**: send Kimi a unique word, quit Sonata, reopen, ask for the word.
   Then `/sessions` to another conversation, send a message, reopen: the other conversation
   comes back. Delete the session folder under `~/.kimi-code/sessions/`, reopen: one notice, fresh
   conversation. Record the exit status of `kimi --session session_missing` (research R6).
6. **Remove**: Settings › Remove → confirm. The block is gone, the rest of the file is unchanged,
   the status is off, and a second backup exists.
7. **Known gap**: `/clear` inside Kimi, then ask "what is Sonata?". Expected: Kimi does not know
   (research R2). Start fresh (terminal menu): it knows again. With the setup on, a plain Restart
   resumes the cleared conversation and still does not know.
8. **Environment (US5)**: `KIMI_PLUGIN_ROOT=/x open -n Sonata.app`; in a terminal, `env | grep KIMI`
   shows no `KIMI_PLUGIN_ROOT`; a user-set `KIMI_CODE_HOME` is kept.
