# Research: Session Restore for Agent Terminals

**Feature**: 009-session-restore | **Date**: 2026-10-24 · Checked against the local Claude Code
v2.1.310, the current `core` and `cli` sources (`terminal/pty.rs`, `messaging/instructions.rs`,
`messaging/commands.rs`, `cli/src/run.rs`, `cli/src/hook.rs`, `app.rs`), and the environment of a
Claude Code child process on this Mac. Codex is not installed; Codex facts come from public
sources and are marked "unverified live".

## R1. Learning the active conversation (Claude Code)

**Decision**: Sonata learns the conversation id from the hooks it already passes to Claude Code
through the session settings file (feature 003, research R4/R5):

- Add a `SessionStart` hook: `sonata _hook session-start`. It reads the hook payload from stdin
  (`session_id`, `source`). `source` is `startup`, `resume`, `clear`, or `compact`.
  - `resume`: the conversation has messages → report it as the terminal's conversation.
  - `startup` or `clear`: a new, empty conversation → report "no conversation" (forget the stored
    one), so an empty conversation is never resumed (FR-004).
  - `compact`: same id, nothing to report.
- Extend the existing `UserPromptSubmit` hook (`_hook turn-start`): it already runs on every user
  message; it now also reads `session_id` from stdin and reports it. This records a new
  conversation at its first message (FR-003, FR-004). The app ignores a report that matches what
  it already stores, so the steady state costs one local socket message per prompt and no disk
  write.

**Rationale**: No new channel, no file scanning, nothing written to the agent's configuration
(FR-005). The hooks run inside the terminal's process tree, so the existing per-terminal identity
(`SONATA_TERMINAL_ID` + socket) tells the app which node they belong to. `/clear`, `/resume`, a
new `claude` typed by hand, and Sonata's own resume all fire one of these two hooks.

**Alternatives considered**: Choosing the id up front with `--session-id <uuid>`: does not follow
`/clear` or `/resume`, so hooks are needed anyway. Watching `~/.claude/projects/*/` for the newest
file: guesses wrong when two agents share a folder (the user's own setup in 006–008). Reading the
`CLAUDE_CODE_SESSION_ID` environment variable from the hook: present today, but the hook payload
is the documented source.

## R2. Learning the active conversation (Codex)

**Decision**: Read `thread-id` from the payload of the `notify` command Sonata already passes
(`-c notify=…`, event `agent-turn-complete`) and report it. Codex calls `notify` only at the end
of a turn, so a Codex conversation is recorded after its first completed turn, and a new
conversation started inside Codex is recorded at the end of its first turn. Unverified live: that
`thread-id` is the id `codex resume <id>` accepts. The quickstart includes this check for when
Codex is installed.

**Rationale**: Same channel as today's end-of-turn report; nothing written to Codex's
configuration.

**Alternatives considered**: Scanning `~/.codex/sessions/`: same guessing problem as R1.

## R3. Resuming

**Decision**: `sonata run` asks the app for session files with `_session` (feature 003). The reply
gains an optional `resume` object `{ "agent", "id" }`. The app offers it only:

1. when the node stores a conversation for the same agent, and
2. on the first `_session` request after the terminal's process started (app reopen, workspace
   opened, Restart). The offer is consumed by that request. Exiting the agent and typing it again
   in the same terminal starts a new conversation, as the user expects.

The wrapper then builds the command:

- Claude Code: `claude --resume <id> …`.
- Codex: `codex resume <id> …`.

The wrapper does not add a resume when the user's arguments already choose a conversation or are
not an interactive session (FR-008): Claude `-r`, `--resume`, `-c`, `--continue`,
`--session-id`, `--from-pr`, `--teleport`, `-p`, `--print`, or any subcommand; Codex any first
argument that is a subcommand (`resume`, `exec`, `login`, …). In that case the offer is still
consumed.

**Rationale**: The resume decision stays in `core` (who owns the stored id and the terminal's
lifecycle); the wrapper only translates it into the agent's flags. Agents typed by hand in a plain
shell terminal resume too, because they also start through `sonata run` (zsh helpers, feature
003); a plain shell still never starts an agent on its own (FR-011).

**Alternatives considered**: Rewriting the node's launch command on restore: does not cover
agents typed by hand, and changes a setting the user wrote.

## R4. When resume fails

**Decision**: When the wrapper adds a resume, it runs the agent as a child process instead of
replacing itself (`exec`), and waits. While waiting it ignores `SIGINT` and `SIGQUIT`, as
`system(3)` does, so Ctrl+C reaches only the agent. If the agent exits with a non-zero status
within 10 seconds, the resume is treated as failed: the wrapper prints one line,
`sonata: the previous conversation could not be resumed; starting a new one.`, and `exec`s the
agent without the resume. Otherwise the wrapper exits with the agent's status. Without a resume,
the wrapper keeps today's `exec`.

Measured on Claude Code v2.1.310: `claude --resume <unknown id>` prints
`No conversation found with session ID: <id>` and exits with status 1 at once.

The fresh conversation fires `SessionStart` with `source=startup`, which clears the stored
conversation (R1), so the failed id is never tried again (FR-009).

**Rationale**: Covers every cause (files deleted, agent update, folder changed after land) with one
rule and no knowledge of the agent's storage layout. A normal session lasts far longer than
10 seconds; a user who quits within 10 seconds gets exit status 0.

**Alternatives considered**: Checking the conversation file before resuming: needs the agent's
private folder layout and misses the folder-change case. A shell snippet (`claude --resume X ||
claude`): would also start a new conversation after a normal session that ends with an error.

## R5. Clean terminal environment

**Decision**: `core::terminal::pty::spawn` removes an explicit list of agent session markers
before applying Sonata's own variables. The list, from the environment of a Claude Code child
process on this Mac:

`CLAUDECODE`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_EXECPATH`, `CLAUDE_CODE_SESSION_ID`,
`CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_SESSION_ATTENDED`, `CLAUDE_CODE_MESSAGING_SOCKET`,
`CLAUDE_CODE_MESSAGING_TOKEN`, `CLAUDE_PID`, `CLAUDE_EFFORT`,

plus Codex's sandbox markers `CODEX_SANDBOX`, `CODEX_SANDBOX_NETWORK_DISABLED` (unverified live).
The list is one constant used by the PTY spawn and by the 006 usage runner, which also starts
`claude`. User settings such as `CLAUDE_CODE_USE_BEDROCK` or `ANTHROPIC_*` are not on the list
and are kept (FR-016).

**Rationale**: Fixes the "Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION marker"
warning the user saw in a test build. It also stops `CLAUDE_CODE_MESSAGING_TOKEN`, a secret of the
outer agent, from leaking into every terminal (constitution V).

**Alternatives considered**: Removing every `CLAUDE_CODE_*` variable: would drop user settings.
Starting terminals with an empty environment: breaks users' PATH and tools.

## R6. Storing the conversation

**Decision**: New optional field `TerminalContent.conversation` (see data-model.md). A report that
changes the field updates the node, notifies the UI (`NodesChanged`), and saves that workspace at
once instead of waiting for autosave (default every 30 s), so a crash right after a conversation
switch still restores it (US1 scenario 2). A report that matches the stored value does nothing.

No schema bump: the field is optional, and an older Sonata that drops the unknown field on save
only loses the restore, never other data.

**Rationale**: Files are the source of truth (constitution IV); a conversation switch is rare, so
the extra write costs nothing.

## R7. Start fresh

**Decision**: Terminal menu item "Start fresh", shown when the node has a conversation. It calls
the shell command `terminal_start_fresh(node, confirmed)`. `core` checks busy exactly like
`restart_terminal` (`TerminalBusy`, the UI shows the same confirmation), clears the conversation,
saves, and restarts the terminal. The agent's own files are not touched (FR-014).

## R8. Copies of terminals

**Decision**: Sonata has no duplicate, clone, or import of terminals today. The rule "a copied
terminal starts without a conversation" (FR-010) is recorded as a model rule in data-model.md and
enforced by a unit test on the node copy helper used for new nodes; features that add copying
(templates, clone) must clear the field.

## R9. Print-mode runs in a terminal

**Decision**: When the wrapper starts Claude Code in print mode (`-p`/`--print`), it sets
`SONATA_CONVERSATION=off` for the child. The hook commands see it and skip conversation reports,
so a one-shot `claude -p …` typed in a terminal does not replace or clear the interactive
conversation.
