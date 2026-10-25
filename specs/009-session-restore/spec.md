# Feature Specification: Session Restore for Agent Terminals

**Feature Branch**: `009-session-restore`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "When Sonata reopens (app restart, crash, or Mac reboot), each agent
terminal (Claude Code, Codex) comes back in the same conversation it had before, instead of a
fresh empty session. Today the user must find the session file by hand and type /resume <id> in
each terminal. Sonata remembers each terminal's conversation, keeps it current when the agent
switches conversation inside the terminal, and resumes it on restore. If the conversation no
longer exists, the terminal starts fresh with a short notice. Default is to resume automatically;
the terminal menu offers Start fresh. Plain shell terminals are out of scope. Included fix:
terminals inherit agent-session markers from whoever launched Sonata, which turns off
conversation saving inside them, so there is nothing to resume; strip those markers for every
terminal."

Builds on 001 (terminal nodes, saved scrollback, restart), 003 (agents started through Sonata
with session instructions and end-of-turn reports), and 008 (terminals in lanes).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Agents come back in their conversation after Sonata reopens (Priority: P1)

A developer has three agent terminals working on three features. They quit Sonata, or the Mac
restarts. When they open Sonata again, each agent terminal starts its agent inside the same
conversation it had before. The agent remembers what it was doing, and the developer can type
"continue" without explaining the task again. The developer does nothing to make this happen.

**Why this priority**: This is the reason for the feature. Today every restart loses the agents'
context, and the user must find each conversation by hand and resume it in each terminal.

**Independent Test**: Start an agent terminal, send it a message with a unique word, and quit
Sonata. Reopen Sonata. Ask the agent which word it was given; it answers with that word.

**Acceptance Scenarios**:

1. **Given** an agent terminal that has had at least one exchange, **When** Sonata quits normally
   and reopens, **Then** the terminal starts the same agent in the same conversation, in the same
   folder.
2. **Given** an agent terminal in a conversation, **When** Sonata stops unexpectedly (crash, force
   quit, power loss) and reopens, **Then** the terminal resumes the conversation that was last
   recorded before the stop.
3. **Given** several agent terminals in several workspaces, **When** Sonata reopens, **Then** each
   terminal resumes its own conversation and never another terminal's.
4. **Given** an agent terminal that was started but never received a message, **When** Sonata
   reopens, **Then** the terminal starts a fresh conversation without any notice, because there
   was nothing to resume.
5. **Given** an agent terminal that the user restarts from its menu (Restart), **When** it starts
   again, **Then** it resumes the same conversation, as on app reopen.

---

### User Story 2 - Conversation saving works in every terminal (Priority: P1)

A developer opens Sonata from a place that already runs an agent, for example a test build that
an agent launched, or a terminal inside another agent session. Today, agents started in that
Sonata believe they are part of the outer agent's session and do not save their conversations,
so nothing can be resumed. After this feature, every terminal Sonata opens starts clean, and each
agent inside it saves its conversation normally.

**Why this priority**: Without this fix, User Story 1 silently fails for these terminals. The user
already hit this problem ("Transcript saving is off — inherited CLAUDE_CODE_CHILD_SESSION
marker").

**Independent Test**: Launch Sonata from an environment that carries an agent's session markers.
Open a terminal and list its environment: none of those markers are present. Start an agent: it
does not warn that conversation saving is off.

**Acceptance Scenarios**:

1. **Given** Sonata was launched from an environment that contains an agent's session markers,
   **When** any terminal starts (agent or plain shell), **Then** those markers are absent from the
   terminal's environment.
2. **Given** the user's own environment settings that are not agent session markers, **When** a
   terminal starts, **Then** those settings are kept unchanged.

---

### User Story 3 - The remembered conversation follows the agent (Priority: P2)

A developer is talking to an agent and, inside the terminal, clears the conversation, or switches
to an older conversation with the agent's own resume command. Sonata notices and remembers the
conversation that is now active. On the next reopen the terminal comes back in the conversation
the developer was last using, not the one it started with.

**Why this priority**: Users change conversations inside agents often. Without this, restore
would bring back an old or empty conversation and feel broken, but the main restore (US1) still
gives value for terminals that keep one conversation.

**Independent Test**: In an agent terminal, start a new conversation from inside the agent and
send it a unique word. Quit and reopen Sonata. The agent knows the new word, not the first one.

**Acceptance Scenarios**:

1. **Given** an agent terminal, **When** the user starts a new conversation inside the agent,
   **Then** Sonata remembers the new conversation for that terminal.
2. **Given** an agent terminal, **When** the user resumes a different, older conversation inside
   the agent, **Then** Sonata remembers that conversation for that terminal.
3. **Given** the user exits the agent and starts it again by hand in the same terminal, **When**
   the new agent session begins, **Then** Sonata remembers the new conversation.

---

### User Story 4 - Start fresh on purpose (Priority: P2)

A developer finished a task and wants the agent in a terminal to start with no memory of it. They
choose "Start fresh" in the terminal's menu. Sonata forgets the remembered conversation and
restarts the terminal's agent in a new, empty conversation. The old conversation still exists for
the agent's own history tools; Sonata only stops resuming it.

**Why this priority**: Automatic resume is the default, so users need a clear way out. It is less
frequent than restore itself.

**Independent Test**: In a terminal with a remembered conversation, choose Start fresh. The agent
restarts with an empty conversation. Quit and reopen Sonata: the terminal resumes the new
conversation, not the old one.

**Acceptance Scenarios**:

1. **Given** an agent terminal with a remembered conversation, **When** the user chooses Start
   fresh, **Then** the agent restarts in a new conversation in the same folder.
2. **Given** an agent terminal with no remembered conversation, **When** the user opens the
   terminal menu, **Then** Start fresh is not offered, or is shown disabled.
3. **Given** the agent is in the middle of a reply, **When** the user chooses Start fresh,
   **Then** Sonata asks for confirmation before stopping it.

---

### User Story 5 - Restore fails gracefully (Priority: P3)

The remembered conversation can no longer be resumed: its files were deleted, the agent was
updated and dropped old conversations, or the terminal's folder changed. On reopen, the terminal
starts a fresh conversation and shows one short notice that the previous conversation could not
be resumed. The terminal never loops trying to resume.

**Why this priority**: Rare, but a failed resume must never leave a terminal stuck or repeating
errors.

**Independent Test**: Remember a conversation, delete its files by hand, and reopen Sonata. The
terminal shows the notice once and is ready for input in a fresh conversation.

**Acceptance Scenarios**:

1. **Given** a remembered conversation that no longer exists, **When** the terminal starts,
   **Then** it starts a fresh conversation and shows a one-line notice in the terminal.
2. **Given** a resume that failed, **When** the fresh conversation begins, **Then** Sonata
   remembers the fresh conversation and does not try the failed one again.
3. **Given** the agent program is not installed any more, **When** the terminal starts, **Then**
   the terminal shows the same error it shows today for a missing agent and does not retry.

---

### Edge Cases

- **Duplicated terminal**: a terminal copied or duplicated on the canvas starts a fresh
  conversation. Two terminals never resume the same conversation, because two agents writing to
  one conversation would corrupt it.
- **Workspace cloned or imported**: terminals in the copy start fresh for the same reason.
- **Same conversation opened elsewhere**: if the remembered conversation is already open in
  another program outside Sonata, the resume follows whatever the agent does in that case; Sonata
  does not lock conversations.
- **Folder changed**: a terminal whose folder changed (for example a lane terminal restarted in
  the workspace folder after land) tries to resume. If the agent cannot find the conversation from
  the new folder, the graceful path of User Story 5 applies.
- **Terminal with a custom launch command**: a terminal whose launch command already asks the
  agent to resume or continue a specific conversation keeps the user's command unchanged, and
  Sonata does not add its own resume.
- **Agent started by hand**: an agent typed by hand in a plain shell terminal is remembered too,
  because it starts through Sonata (feature 003). On reopen, a plain shell terminal reopens as a
  shell and does not start the agent by itself.
- **Several agents in one terminal over time**: only the most recent agent session is remembered.
- **Unsupported agents**: agents other than Claude Code and Codex start as they do today.
- **Saved scrollback**: when the terminal also saves its scrollback, the old screen text is shown
  first and the resumed agent draws below it, as today.

## Requirements *(mandatory)*

### Functional Requirements

**Remembering the conversation**

- **FR-001**: For every agent terminal running a supported agent (Claude Code, Codex), Sonata MUST
  record which conversation the agent is using.
- **FR-002**: The recorded conversation MUST be saved with the terminal in the workspace file, so
  it survives app quit, crash, and Mac restart.
- **FR-003**: When the agent switches conversation inside the terminal (new conversation, clear,
  resume of another conversation, agent restarted by hand), Sonata MUST record the new one within
  5 seconds of the switch.
- **FR-004**: A conversation MUST be recorded only after it has at least one message from the
  user, so an empty conversation is never resumed.
- **FR-005**: Recording MUST NOT write anything to the agent's own configuration files or to the
  user's project folder.

**Resuming**

- **FR-006**: When an agent terminal starts (app reopen, terminal Restart, workspace opened), and
  it has a recorded conversation, Sonata MUST start the agent resuming that conversation, with no
  action from the user.
- **FR-007**: Resume MUST keep everything the terminal does today at start: the same folder, the
  same session instructions and end-of-turn reports (feature 003), the same lane (feature 008).
- **FR-008**: If the terminal's launch command already tells the agent to resume or continue a
  conversation, Sonata MUST NOT add its own resume.
- **FR-009**: If resuming fails, the terminal MUST start a fresh conversation, show one short
  notice that says the previous conversation could not be resumed, and record the fresh
  conversation. Sonata MUST NOT retry the failed conversation.
- **FR-010**: A duplicated terminal, and every terminal in a cloned or imported workspace, MUST
  start without a recorded conversation.
- **FR-011**: Plain shell terminals MUST reopen as today (same folder, saved scrollback if
  enabled) and MUST NOT start an agent on their own.

**Start fresh**

- **FR-012**: The terminal menu MUST offer "Start fresh" for agent terminals with a recorded
  conversation. It forgets the recorded conversation and restarts the agent in a new one.
- **FR-013**: Start fresh MUST ask for confirmation when the agent is busy, using the same
  confirmation Sonata uses today before stopping a busy terminal.
- **FR-014**: Start fresh MUST NOT delete the agent's conversation files.

**Clean terminal environment**

- **FR-015**: Every terminal Sonata starts MUST NOT inherit environment variables that mark an
  agent session (for Claude Code: the variables that identify a running or child session), so
  that agents inside save their conversations normally.
- **FR-016**: The list of removed variables MUST be explicit and limited to agent session
  markers. All other inherited variables MUST be kept.

### Key Entities

- **Remembered conversation**: belongs to one terminal node. Holds which agent it belongs to
  (Claude Code or Codex), the agent's identifier for the conversation, and when it was last
  recorded. Saved in the workspace file with the terminal; omitted when the terminal has none.
- **Agent session marker**: an environment variable an agent sets for programs it starts, which
  makes a nested agent behave as a child session. Removed from every terminal's environment.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After Sonata quits and reopens, 100% of agent terminals that had a recorded
  conversation resume it, with zero manual steps by the user.
- **SC-002**: Restoring a workspace with 3 agent terminals takes the user no extra action and no
  more than 10 seconds longer than starting the same terminals fresh.
- **SC-003**: After switching conversation inside an agent and reopening Sonata, the terminal
  resumes the conversation that was active last, in 100% of tested switches (new, clear, resume
  of another conversation, agent restarted by hand).
- **SC-004**: A conversation that cannot be resumed produces exactly one notice and a usable
  fresh conversation; no terminal is left without a prompt.
- **SC-005**: No terminal started by Sonata shows a "conversation saving is off" warning caused
  by inherited session markers.

## Assumptions

- Supported agents are Claude Code and Codex, the two agents Sonata already starts with session
  instructions (feature 003). Codex is not installed on the development Mac, so Codex behavior is
  verified with fakes and the agent's documented commands, as in feature 006.
- Both agents offer a way to resume a conversation by its identifier, and a way for Sonata to learn
  the identifier of the active conversation through the session hooks and notifications it already
  receives. The plan confirms the exact mechanism for each agent.
- The conversation files belong to the agent and stay in the agent's own folders. Sonata stores
  only the identifier.
- Automatic resume is the default and has no global switch in this feature; Start fresh is the
  per-terminal way out.
- Restoring conversations for agents running inside device nodes, browser nodes, or remote hosts
  is out of scope.
