# Research: Agent Messaging over Links, Sonata CLI, and Orchestrator Powers

**Feature**: 003-agent-messaging | **Date**: 2026-10-22

Agent facts were checked on 2026-10-22: Claude Code 2.1.307 `--help` on this machine; Codex from
its official CLI reference (Codex is not installed here, see R4).

## R1. Channel: Unix socket with newline-delimited JSON

**Decision**: The app listens on a Unix socket at `<data root>/run/sonata.sock` (folder `0700`,
socket `0600`). If that path is longer than the macOS limit (104 bytes, e.g. a deep
`SONATA_HOME` in tests), the socket goes to `$TMPDIR/sonata-<uid>/<hash>.sock` in a `0700` folder.
One request per connection: the CLI writes one JSON line `{v, terminalId, token, cwd, args}` and
reads JSON lines back (`progress` lines while waiting, then one `result` line).

**Rationale**: Local only, private by file mode (FR-002), no port to guard. A request/response
of JSON lines is all the CLI needs; HTTP adds a dependency and nothing else.

**Alternatives considered**: HTTP over the socket with `hyper` (named in the constitution's
stack; dropped by amendment 1.1.0 for simplicity); TCP on localhost (open to every local process,
needs a port).

## R2. Identity

**Decision**: Each terminal start creates a random 32-byte token. The shell gets
`SONATA_TERMINAL_ID`, `SONATA_TOKEN`, `SONATA_SOCKET`, and the CLI folder on `PATH`. The server
accepts a request only if the token matches the terminal's current session; restarting a terminal
creates a new token. Tokens are never written to disk.

**Threat model**: stops agents (and mistakes) from acting as another terminal. It does not defend
against malicious code running as the same macOS user, which could read the socket and process
environments anyway; Sonata already runs unsandboxed (constitution V).

## R3. The `sonata` CLI

**Decision**: New crate `crates/cli`, one small synchronous binary (`std::os::unix::net`,
`serde_json`, no async runtime). Shipped inside the app at `Contents/Resources/bin/sonata`
(built by `beforeBuildCommand`, copied with `bundle.resources`). In development the app uses
`target/debug/sonata`. Without `SONATA_SOCKET` it prints that it only works inside Sonata (FR-001).

Exit codes: 0 success, 2 usage, 3 permission, 4 not found, 5 timed out, 6 app not reachable,
1 other.

## R4. Agent sessions: `sonata run <agent>`

**Decision**: When a terminal's launch command starts with `claude` or `codex`, Sonata types
`sonata run claude <rest>` instead. `sonata run` writes nothing to the user's agent configuration;
it execs the real agent with session-only options:

- **Claude Code**: `--append-system-prompt-file <run>/<id>/instructions.md` and
  `--settings <run>/<id>/settings.json`, where the settings contain only hooks: `Stop` →
  `sonata _hook claude-stop` and `UserPromptSubmit` → `sonata _hook turn-start`.
- **Codex**: `-c 'notify=["sonata","_hook","codex-notify"]'`. Session instructions for Codex use
  the typed fallback (R6) until the instructions key is verified on an installed Codex; the
  config key name could not be confirmed from official docs on 2026-10-22.

The user can also type `sonata run claude` by hand in any Sonata terminal. A plain `claude`
typed by hand works as before, without Sonata awareness.

**Rationale**: PATH shims named `claude` would lose to user `PATH` changes made in `.zshrc`;
an explicit `sonata run` is reliable and visible.

## R5. Knowing a turn finished, and the answer

**Decision**:
- Claude Code: the `Stop` hook receives JSON on stdin with `transcript_path`; `sonata _hook
  claude-stop` reads the last assistant text from that transcript and sends `turn_complete
  {text}` for its terminal.
- Codex: the notify program receives JSON as its last argument (`type: agent-turn-complete`,
  `last-assistant-message`); `sonata _hook codex-notify` forwards it.
- Others (heuristic, FR-017): after sending, wait for output; the turn ends after 3 s without new
  output once some output came and the terminal is not running a child program beyond the agent;
  30 s of total silence also ends it.
- The answer for the heuristic path is the text produced since sending, taken from the core screen
  model: at send time the core stores an absolute row index (scrolled-off rows are counted with
  `vt100`'s `on_scroll` callback); at the end it returns the rows from that index to the last
  non-empty row, as plain text. For a full-screen (alternate screen) program it returns the
  visible screen and says so.
- A `sonata reply` always wins (FR-008).

## R6. Instructions content and typed fallback

**Decision**: `instructions.md` is generated per start from a template in `crates/core`: what
Sonata is, the terminal's name and workspace, the role text, the list of `sonata` commands with
one example each, and the rule "when someone asks you through Sonata, answer normally; for long
answers run `sonata reply`". The typed fallback sends a shorter version as a pasted message once
the agent is ready (quiet 2 s after start), only when the terminal's option is on.

## R7. Delivering a message

**Decision**: If the target's screen has bracketed paste on, send `ESC[200~` + text + `ESC[201~`,
wait 80 ms, then `\r`; otherwise send the text with line breaks as `\r` and a final `\r`. Keys for
`--keys`: `enter`, `tab`, `esc`, `up`, `down`, `left`, `right`, `ctrl-c`, `ctrl-d`, `space`,
`backspace`, and literal text in quotes.

## R8. Broker

**Decision**: New `crates/core/src/messaging/` with an `AskBroker` per app: queue per target
(FIFO), wait graph for loop detection (refuse when the target is waiting, directly or through a
chain, for the asker), deadlines, cancellation on close/exit/quit, and an in-memory log of the last
50 exchanges per terminal. Waiting happens on a `std::sync::mpsc` channel per ask; the socket
handler thread blocks on it and sends `progress` lines every 15 s.

## R9. Starting a target on demand

**Decision**: Asking a terminal of an unloaded workspace loads it (starts its terminals, without
switching the UI). The message waits until the target is ready: 2 s quiet after its first output,
or the agent's `turn-start`/ready signal when available, at most 60 s.

## R10. Recruit placement

**Decision**: Try positions to the right of the orchestrator, then below, then left, on a 40 px
grid, and pick the first 640×400 rectangle that does not overlap any node (with 24 px margin).

## R11. Notifications and single instance

**Decision**: `tauri-plugin-notification` for `sonata notify`; on click the system activates the
app, and the UI selects the last notifying terminal when the window gains focus within 60 s.
`tauri-plugin-single-instance` focuses the running app on a second launch (FR-031).

## R12. Visibility

**Decision**: Events `messaging://ask` `{askId, from, to, state}` drive link highlights and the
"asked by" marker; `exchange_log(nodeId)` returns the in-memory log for the toolbar panel.
