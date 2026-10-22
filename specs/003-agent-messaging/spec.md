# Feature Specification: Agent Messaging over Links, Sonata CLI, and Orchestrator Powers

**Feature Branch**: `003-agent-messaging`

**Created**: 2026-10-22

**Status**: Draft

**Input**: User description: "Feature 003 — Agent messaging over links, Sonata CLI, and orchestrator
powers. Agents running in Sonata terminals can see who they are linked to and talk to each other
through links, and an orchestrator agent can build and manage its team."

Builds on 001 (workspaces, terminals, notes) and 002 (links, terminal settings with role and
Orchestrator flag). Sonata uses its own tool name, commands, wording, and agent instructions.

## Clarifications

### Session 2026-10-22 (decided before specification)

- Q: How do agents learn about Sonata and receive their role? → A: Per session, through the
  agent's own startup options; the user's global agent configuration is never changed. Agents
  without such options get a typed instruction message (opt-in per terminal).
- Q: How does Sonata know an agent finished answering? → A: The agent's own session-level
  completion signal for known agents, with a quiet-terminal heuristic as fallback for all others.
- Q: Which orchestrator powers are in scope? → A: recruit/dismiss, link/unlink, notes for every
  agent, and system notifications. The browser node is feature 004.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - One agent asks another and gets the answer (Priority: P1)

A developer links a "Lead" Claude Code terminal to a "Backend" Codex terminal. They tell Lead:
"ask Backend which endpoints are missing tests". Lead runs `sonata ask Backend "…"`; the question
appears in Backend's terminal as if typed, Backend works, and when it finishes Lead receives
Backend's answer as text and continues.

**Why this priority**: This is the reason links exist. Without it Sonata is a canvas of isolated
terminals.

**Independent Test**: Two linked terminals running plain shells. In A, run `sonata ask B "echo
hello"`; A prints `hello`. Unlink them; the same command fails with a clear message.

**Acceptance Scenarios**:

1. **Given** terminals A and B linked, **When** A runs `sonata ask B "<message>"`, **Then** the
   message is entered in B exactly as written (multi-line text arrives as one message, not line by
   line) followed by Enter, and A's command waits.
2. **Given** an ask is waiting, **When** B's agent finishes its turn, **Then** A's command prints
   what B produced after the message was sent, without color codes or screen-drawing artifacts,
   and exits successfully.
3. **Given** an ask is waiting, **When** B runs `sonata reply "<text>"`, **Then** A's command
   prints exactly that text at once and exits successfully.
4. **Given** an ask is waiting, **When** the timeout passes (default 10 minutes, set per call up to
   60 minutes), **Then** A's command prints what B produced so far, says B is still working, and
   exits with a distinct "timed out" status. The message is never sent twice.
5. **Given** A and B are not linked, **When** A asks B, **Then** the command fails and lists the
   names A is linked to.
6. **Given** A is linked to two terminals named "Worker", **When** A asks "Worker", **Then** the
   command fails and explains the name is ambiguous.
7. **Given** A is linked to B, C, and D, **When** A runs `sonata ask --all "<message>"` or
   `sonata ask B,C "<message>"`, **Then** the message goes to each target at the same time, and A
   receives every answer labeled by name, including which ones timed out or failed.
8. **Given** B shows an interactive menu, **When** A runs `sonata ask B --keys "down enter"`,
   **Then** those keys are sent to B and A's command returns immediately.
9. **Given** A is linked to B, **When** A runs `sonata check B`, **Then** A prints B's current
   visible screen as text, and nothing is sent to B.
10. **Given** B is in a workspace that is not open or its program exited, **When** A asks B,
    **Then** B is started first and the message is sent when B is ready to receive input.

---

### User Story 2 - Agents know where they are and who they work with (Priority: P1)

A developer starts Claude Code from the Claude Code preset with a role. Without being told,
the agent knows it runs inside Sonata, knows its role, and can run `sonata list` to see its
linked terminals and notes.

**Why this priority**: Agents only use the tool if they know about it. The user should not have to
explain Sonata in every session.

**Independent Test**: Start a Claude Code terminal with a role and links. Ask the agent "who are
you linked to and what is your role?" It answers correctly by itself.

**Acceptance Scenarios**:

1. **Given** any Sonata terminal, **When** `sonata list` runs, **Then** it prints this terminal's
   name, workspace, role (if any), whether it is the Orchestrator, and each linked item: terminals
   with name, state (idle, working, exited), and agent kind; notes with their first line.
2. **Given** a terminal starts Claude Code or Codex, **Then** that session receives instructions
   explaining the `sonata` commands and, when set, the terminal's role, without any change to the
   user's global configuration of that agent.
3. **Given** a terminal whose agent has no way to receive instructions at start, and the
   terminal's "send instructions on start" option is on, **Then** Sonata types a short instruction
   message into it once the agent is ready.
4. **Given** the `sonata` command runs outside a Sonata terminal, **Then** it explains that it
   only works inside Sonata and exits with an error.

---

### User Story 3 - An orchestrator builds and runs its team (Priority: P2)

A developer marks one terminal as Orchestrator and asks it to "set up a reviewer and a tester for
this repo". The orchestrator recruits two new agents with roles; they appear next to it, linked to
it, running. It asks them for work, links the tester to a shared plan note, and later dismisses
the reviewer.

**Why this priority**: Turns Sonata from manual wiring into self-organizing teams; depends on US1.

**Independent Test**: An Orchestrator shell runs recruit, link, unlink, and dismiss commands; each
change appears on the canvas and is saved. A non-orchestrator gets an error for each.

**Acceptance Scenarios**:

1. **Given** an Orchestrator terminal, **When** it runs `sonata recruit claude --name Reviewer
   --role Reviewer`, **Then** a new terminal named Reviewer appears next to the orchestrator
   without overlapping other items, linked to it, running the Claude Code preset with the library
   role "Reviewer", and the command prints the new name.
2. **Given** `--role-text "…"` instead of `--role`, **Then** the new terminal gets that text as its
   role. **Given** `--folder F`, **Then** it starts in F (which must exist).
3. **Given** a name already used by a linked terminal, **When** recruiting with it, **Then** Sonata
   adds a number to make it unique and prints the final name.
4. **Given** an Orchestrator, **When** it runs `sonata dismiss Reviewer` for a terminal it
   recruited, **Then** that terminal is closed (its program stopped) and removed with its links.
5. **Given** an Orchestrator, **When** it tries to dismiss a terminal it did not recruit, **Then**
   the command is refused.
6. **Given** an Orchestrator, **When** it runs `sonata link Tester Plan` or `sonata unlink Tester
   Plan` for items in its workspace, **Then** the link is created or removed and shown on the
   canvas.
7. **Given** a terminal without the Orchestrator flag, **When** it runs recruit, dismiss, link, or
   unlink, **Then** the command fails with "only the Orchestrator can do this".

---

### User Story 4 - Agents share notes and notify the user (Priority: P2)

A developer links a "Plan" note to three agents. Each agent reads the plan and appends its status
to it. When the orchestrator finishes, it sends a macOS notification.

**Why this priority**: Notes are the shared memory of a team; notifications let the user leave the
screen.

**Independent Test**: A terminal linked to a note reads it, replaces it, and appends to it; the
note on the canvas updates. A notification appears for `sonata notify`.

**Acceptance Scenarios**:

1. **Given** a terminal linked to a note, **When** it runs `sonata note read Plan`, **Then** it
   prints the note's Markdown.
2. **Given** the same, **When** it runs `sonata note write Plan "<text>"` or `--append "<text>"`,
   **Then** the note is replaced or extended, saved, and the canvas shows the new text within
   2 seconds. If the user is editing that note at the moment, the same conflict rule as edits made
   outside the app applies (001 FR-020a).
3. **Given** a note not linked to the terminal, **Then** read and write fail like unlinked asks.
4. **Given** notes have no names, **Then** a note is addressed by its first heading or first line
   (as shown by `sonata list`), or by the short id that `sonata list` prints.
5. **Given** any terminal, **When** it runs `sonata notify "<message>"`, **Then** macOS shows a
   notification titled with the terminal's name and the message; clicking it brings Sonata to the
   front with that terminal selected.

---

### User Story 5 - The user sees the conversation (Priority: P3)

While agents talk, the developer sees which link is busy, which terminal is being asked and by
whom, and can open a short log of recent messages for any terminal.

**Why this priority**: Trust and debugging; the system works without it.

**Independent Test**: Run several asks; watch the link highlight and the "asked by" marker; open
the log and see each exchange.

**Acceptance Scenarios**:

1. **Given** an ask is sent, **Then** the link between the two terminals is highlighted until the
   answer returns.
2. **Given** a terminal is being asked, **Then** its node shows "asked by <name>" until it answers.
3. **Given** a terminal, **When** the user opens its message log from the toolbar, **Then** it shows
   the last 50 exchanges it took part in: time, direction, other party, the first line of the
   message, outcome (answered, replied, timed out, failed), and answer length.

---

### Edge Cases

- A asks B while B is already answering another ask: the new message is queued and sent after B
  finishes the current one; the queue position is shown in the waiting command's output.
- A asks B, and B (during that ask) asks A: refused with a "loop" error naming the chain, because
  A is waiting and cannot answer.
- B exits or is closed while an ask is waiting: the ask ends at once with "B stopped" and whatever
  B produced.
- A is closed while waiting: the ask is cancelled; B's turn continues normally.
- `sonata reply` when nobody is waiting for this terminal: it fails with "no pending question".
- Messages over 100 KB are refused; answers over 1 MB are cut with a note saying so.
- The app quits while asks are waiting: waiting commands end with "Sonata quit".
- A recruited terminal's preset program is not installed: the terminal is still created, shows
  the shell's "command not found", and recruit reports it.
- Two Sonata windows or apps are not supported; a second instance of the app refuses to start
  while the first runs.
- An agent produces a full-screen interface (alternate screen): the answer is its last visible
  screen instead of the output since sending, and the result says so.

## Requirements *(mandatory)*

### Functional Requirements

**Tool and channel**

- **FR-001**: Every Sonata terminal MUST have a `sonata` command available without setup. Outside
  Sonata terminals it MUST explain that it only works inside Sonata and fail.
- **FR-002**: The command MUST talk to the running app over a channel only the user can use; there
  MUST be no network listener. Each terminal MUST carry a secret identity known only to that
  terminal and the app, so one terminal can never act as another.
- **FR-003**: Every command MUST print human-readable text by default and JSON with `--json`, and
  MUST exit with distinct statuses for success, usage error, permission error, not found, timed
  out, and app not reachable.

**Permissions**

- **FR-004**: A terminal MUST only ask, check, read, or write items it is linked to. Names MUST be
  resolved among linked items, matching case-insensitively; no match or several matches MUST fail
  with a message listing the linked names.
- **FR-005**: recruit, dismiss, link, and unlink MUST be refused unless the calling terminal has the
  Orchestrator flag. dismiss MUST only work on terminals recruited by that same orchestrator.
  link and unlink MUST only work within the orchestrator's workspace.

**Asking**

- **FR-006**: ask MUST deliver the message to the target as one pasted block followed by Enter,
  preserving line breaks, and MUST wait for the target to finish.
- **FR-007**: The ask result MUST be the target's output produced after the message was sent, as
  plain text (color and cursor codes removed, redrawn lines collapsed to their final state).
- **FR-008**: A `sonata reply` from the target MUST complete the oldest pending ask to it
  immediately with the reply text as the result.
- **FR-009**: Ask MUST time out after 10 minutes by default; `--timeout` MUST accept 5 seconds to
  60 minutes. On timeout it MUST return the partial result, state that the target is still working,
  and MUST NOT resend.
- **FR-010**: Asking several targets (`--all` or a comma list) MUST send in parallel and return one
  labeled result per target.
- **FR-011**: `--keys` MUST send named keys and plain characters without waiting.
- **FR-012**: check MUST return the target's visible screen as plain text without sending input.
- **FR-013**: A target that is busy with another ask MUST receive new asks in order, one at a time.
- **FR-014**: An ask that would close a waiting loop MUST be refused with a message naming the
  chain.
- **FR-015**: A target that is not running MUST be started (with its workspace's terminals) before
  the message is sent; the message is sent once the target is ready for input.

**Finishing detection**

- **FR-016**: For Claude Code and Codex, Sonata MUST use the agent's own end-of-turn signal,
  configured for that session only.
- **FR-017**: For every other program, an ask MUST end when the terminal has produced no new
  output for 3 seconds after having produced some, and no program other than the agent's own is
  running in the foreground; an ask whose target never produces output ends after 30 seconds of
  silence.

**Agent awareness**

- **FR-018**: A terminal starting Claude Code or Codex MUST pass that session instructions that
  describe the `sonata` commands, the terminal's name, and its role text when set. The user's
  global agent configuration MUST NOT be changed.
- **FR-019**: Each terminal MUST have a "send instructions on start" option (default on for
  agents without session instructions, off for plain shells) that types the instruction message
  once the agent is ready.
- **FR-020**: `sonata list` MUST print the caller's identity and linked items as described in User
  Story 2.

**Orchestrator**

- **FR-021**: recruit MUST create a terminal next to the orchestrator without overlapping other
  items, link it to the orchestrator, record who recruited it, start it with the preset or command
  given, apply `--name`, `--role` (library name), `--role-text`, and `--folder`, make the name
  unique among the orchestrator's links, and print the final name.
- **FR-022**: dismiss MUST stop and remove a terminal the orchestrator recruited, with its links.
- **FR-023**: link and unlink MUST create or remove a link between two named items of the
  orchestrator's workspace, resolved by name among all items of that workspace.

- **FR-021a** (added 2026-10-22 from user testing): `sonata note new "<title>" ["<text>"]` and
  `sonata browser new [<url>]` MUST create the item next to the Orchestrator, linked to it.
- **FR-021b** (added 2026-10-22): `--help` or `-h` on any command MUST only print help and never
  run the command; unknown options MUST be refused.
- **FR-018a** (added 2026-10-22): In zsh terminals, `claude` and `codex` typed by hand MUST start
  through `sonata run` (session instructions), while the user's own zsh startup files keep loading
  unchanged.

**Notes and notifications**

- **FR-024**: note read, write, and write `--append` MUST work on linked notes, update the canvas
  within 2 seconds, and follow the conflict rule of 001 FR-020a while the user edits the note.
- **FR-025**: Notes MUST be addressable by their title (first heading, else first line) or by the
  short id printed by `sonata list`.
- **FR-026**: notify MUST show a macOS notification titled with the terminal's name; activating it
  MUST bring Sonata forward with that terminal selected.

**Visibility**

- **FR-027**: While an ask waits, the link between the two terminals MUST be highlighted and the
  target node MUST show "asked by <name>".
- **FR-028**: Each terminal MUST keep its last 50 exchanges (time, direction, other party, first
  line, outcome, answer length) in memory and show them from its toolbar. The log is not saved to
  disk.

**Limits and lifecycle**

- **FR-029**: Messages over 100 KB MUST be refused; results over 1 MB MUST be truncated with a note.
- **FR-030**: Closing the asking terminal MUST cancel its pending asks; closing or exiting the
  target MUST end pending asks with its partial output; quitting the app MUST end all pending asks
  with "Sonata quit".
- **FR-031**: Only one instance of Sonata may run per user; a second launch MUST bring the first
  forward and exit.

### Key Entities

- **Agent session identity**: the secret that ties a running terminal to its node; created at each
  start, never written to disk.
- **Ask**: from, to, message, sent time, state (queued, waiting, answered, replied, timed out,
  failed, cancelled), result text.
- **Recruitment record**: on a terminal node, the id of the orchestrator that recruited it
  (persisted with the workspace).
- **Exchange log entry**: one completed ask as seen from one terminal (in memory only).
- **Agent kind**: detected from the terminal's command (Claude Code, Codex, other) to choose the
  session instructions and the finishing signal.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For Claude Code and Codex targets, 95% of asks return the complete answer within 2
  seconds after the target visibly finished its turn.
- **SC-002**: Zero asks return an empty or truncated answer while the target is still working,
  over a test set of 50 asks with answers that pause for up to 20 seconds mid-turn.
- **SC-003**: A Claude Code agent started from a preset answers "what can you do in Sonata and
  who are you linked to?" correctly with no explanation from the user.
- **SC-004**: An orchestrator builds a team of 3 recruited, linked, role-bearing agents from one
  instruction, with every terminal visible on the canvas and saved.
- **SC-005**: No command run in one terminal can read, write, or send to an item it is not linked
  to (verified with a test that tries every command against unlinked items and forged identities).
- **SC-006**: Asking 5 terminals at once returns all 5 labeled answers, and the slowest answer
  arrives no later than it would alone plus 1 second.

## Assumptions

- Supported agents with session instructions and finishing signals: Claude Code and Codex. Other
  presets (OpenCode, Gemini CLI) and custom commands use the typed instructions and the heuristic,
  until verified support is added.
- Agent kind is detected from the first word of the terminal's command (`claude`, `codex`, …).
- The `sonata` tool ships inside the app and is put on each terminal's `PATH` by Sonata; nothing
  is installed into the user's system folders.
- Quiet-terminal values: 3 seconds of quiet after output, 30 seconds with no output at all; they
  are app constants in this feature.
- The exchange log lives in memory; a persistent transcript is out of scope.
- Out of scope: browser node (feature 004), SSH/Docker/remote terminals, chat-style view,
  routines, git worktree lanes, remote companion app, changing the user's global agent settings.
