# Feature Specification: Kimi Code CLI as a Supported Agent

**Feature Branch**: `010-kimi-agent`

**Created**: 2026-10-25

**Status**: Draft

**Input**: User description: "Kimi Code CLI as a supported agent in Sonata. Today Sonata starts
Claude Code and Codex through `sonata run` with session instructions, end-of-turn reports, and
(feature 009) conversation restore. Add Kimi Code CLI (`kimi`) as a third supported agent with the
same experience: Sonata session instructions, turn start/end reports so the canvas knows when Kimi
is busy or done, and session restore. Constraint: Kimi has no per-launch way to add hooks or
config, and Sonata must not write to the agent's configuration files on its own. So hooks are
installed once by explicit user opt-in; the hook commands do nothing outside a Sonata terminal.
Without the hooks installed, Kimi still starts in Sonata as a plain agent and Sonata shows that
busy/done tracking and restore are off, with a way to set them up. The plan must find how to
deliver session instructions without breaking resume. Agent-session environment markers of Kimi
are stripped from terminals like Claude's. Kimi is not installed on the development Mac; behavior
is verified with fakes and documented commands, as with Codex."

Builds on 002 (terminal presets), 003 (agents started through Sonata, session instructions,
end-of-turn reports, quiet-terminal fallback), and 009 (session restore, clean terminal
environment). Usage limits for Kimi (feature 006) are out of scope.

## Background

Today a user can already type `kimi` in a Sonata terminal or set it as a custom launch command.
Kimi then behaves like any unknown agent: it gets the typed instruction message if the terminal
opts in, Sonata guesses when it is done from a quiet screen, and its conversation is not restored.
This feature makes Kimi a first-class agent, like Claude Code and Codex.

Kimi differs from the other two agents in one way that shapes this feature: it reads its settings
and hooks only from one user-level configuration file, and it has no option to add them for a
single launch. Claude Code and Codex accept Sonata's hooks per launch, so the user never has to
change anything. For Kimi, the user must approve the integration once.

## Clarifications

### Session 2026-10-25

- Q: May Sonata write the integration lines into Kimi's configuration file, or only show them for
  the user to paste? → A: One-click Apply after explicit confirmation, with a backup of the file,
  adding only Sonata's block; a Remove action undoes it. This is a deliberate, user-confirmed
  exception to feature 003's rule that the agent's global configuration is never changed; that
  rule targets silent changes, and here nothing changes without the user's confirmation.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start Kimi from a preset and have it know Sonata (Priority: P1)

A developer adds a terminal and picks the "Kimi Code" preset. Kimi starts in the terminal's folder.
Without being told, it knows it runs inside Sonata, knows its role, and can list its linked
terminals and notes. Other agents can ask it questions through links, and it can ask them. This
works on the first launch, before the developer sets anything else up.

**Why this priority**: This is the minimum that makes Kimi usable as a team member on the canvas.
It needs no change to Kimi's configuration.

**Independent Test**: Create a terminal with the Kimi Code preset and a role. Ask Kimi "what can you
do in Sonata, and what is your role?". It names Sonata's commands and its role. From a linked
Claude Code terminal, ask it a question; the answer comes back.

**Acceptance Scenarios**:

1. **Given** the new-terminal dialog, **When** the user opens the preset list, **Then** "Kimi Code"
   is offered next to the existing presets, and choosing it sets the launch command to start Kimi.
2. **Given** a Kimi terminal with a role, **When** Kimi starts a new conversation, **Then** it has
   Sonata's session instructions and its role, without the user typing them.
3. **Given** a Kimi terminal without the integration set up, **When** another agent asks it a
   question, **Then** the answer is returned when Kimi's screen goes quiet, as for other agents
   without completion signals today.
4. **Given** Kimi is not installed, **When** a Kimi terminal starts, **Then** the terminal shows the
   same "not found" error Sonata shows today for a missing agent.

---

### User Story 2 - Set up the Kimi integration once (Priority: P1)

A developer opens a Kimi terminal. Sonata shows, in the terminal's header or menu, that busy/done
tracking and conversation restore are off for Kimi, with a "Set up" action. The action explains
what will change in Kimi's configuration and why, shows the exact lines, and lets the developer
apply them. After setup, every Kimi terminal in every workspace has full tracking and restore. The
added lines do nothing when Kimi runs outside Sonata.

**Why this priority**: Without this, User Stories 3 and 4 cannot work, because Kimi offers no
other way to receive Sonata's hooks.

**Independent Test**: With Kimi's configuration file holding the user's own settings, run the setup.
The user's settings are unchanged, Sonata's lines are present, and Sonata shows the integration as
on. Run `kimi` in a normal terminal outside Sonata: it behaves exactly as before setup.

**Acceptance Scenarios**:

1. **Given** the integration is not set up, **When** a Kimi terminal is shown, **Then** Sonata
   shows a short, dismissible notice that tracking and restore are off, with a Set up action.
2. **Given** the user chooses Set up, **When** the setup view opens, **Then** it shows the exact
   lines Sonata needs, the file they belong in, and what each one does.
3. **Given** the setup view, **When** the user confirms Apply, **Then** Sonata saves a backup of
   Kimi's configuration file, adds only its own block, and shows the integration as on. A Remove
   action in the same view takes the block out again, also after confirmation.
4. **Given** the integration is set up, **When** Kimi runs in a terminal outside Sonata, **Then**
   the added hooks do nothing visible and do not slow Kimi down noticeably.
5. **Given** the integration is set up, **When** the user opens the setup view again, **Then** it
   shows the integration as on and explains how to remove it.
6. **Given** Kimi's configuration file contains lines from an older Sonata version, **When** a Kimi
   terminal is shown, **Then** Sonata shows the integration as out of date, with the same Set up
   action.

---

### User Story 3 - Sonata knows exactly when Kimi is busy and done (Priority: P2)

With the integration set up, a developer sends a message to a Kimi terminal from a linked agent.
The canvas shows Kimi as working when the message is submitted, and the asking agent receives the
answer when Kimi finishes its turn, not after a guessed quiet period. Long pauses during a reply
(for example while a tool runs) do not cut the answer short.

**Why this priority**: It makes asks to Kimi reliable and as fast as asks to Claude Code and Codex.
The quiet-screen fallback from User Story 1 already works, so this is an improvement, not a
blocker.

**Independent Test**: With the integration set up, ask Kimi to run a command that prints nothing
for 20 seconds and then answer. The asking agent receives the full answer once Kimi finishes, not
partial output after the quiet period.

**Acceptance Scenarios**:

1. **Given** the integration is set up, **When** a message is submitted to Kimi, **Then** the
   terminal shows as busy at once.
2. **Given** a pending ask to Kimi, **When** Kimi ends its turn, **Then** the ask returns Kimi's
   answer within 2 seconds.
3. **Given** a pending ask to Kimi, **When** Kimi is silent mid-turn for longer than the quiet
   period, **Then** the ask keeps waiting until Kimi ends its turn.
4. **Given** the integration is set up, **When** Kimi's turn ends with an error or is interrupted,
   **Then** the terminal stops showing as busy and a pending ask returns what Kimi produced.

---

### User Story 4 - Kimi conversations come back after Sonata reopens (Priority: P2)

With the integration set up, Kimi terminals behave like Claude Code and Codex terminals under
feature 009: after Sonata quits, crashes, or the Mac restarts, each Kimi terminal resumes the
conversation it had, follows conversation switches made inside Kimi, offers Start fresh, and
falls back to a fresh conversation with one notice when resume fails.

**Why this priority**: Same value as feature 009 for the other agents, but it depends on User
Story 2 and on feature 009 being in place.

**Independent Test**: With the integration set up, send Kimi a unique word, quit and reopen Sonata,
and ask Kimi which word it was given; it answers with that word.

**Acceptance Scenarios**:

1. **Given** a Kimi terminal that had at least one exchange, **When** Sonata reopens, **Then** the
   terminal resumes the same Kimi conversation in the same folder.
2. **Given** a Kimi terminal, **When** the user switches to another conversation inside Kimi and
   sends a message, **Then** Sonata remembers the new conversation within 5 seconds.
3. **Given** a resumed Kimi conversation, **When** it starts, **Then** Sonata's session instructions
   are still in effect and are not added again as a new message in the conversation.
4. **Given** a remembered Kimi conversation that no longer exists, **When** the terminal starts,
   **Then** Kimi starts fresh with one notice, as in feature 009.
5. **Given** the integration is not set up, **When** Sonata reopens, **Then** Kimi terminals start
   fresh, as today, and the notice from User Story 2 explains why.

---

### User Story 5 - Clean environment for Kimi (Priority: P3)

A developer launches Sonata from inside a Kimi session (for example a test build Kimi started).
Terminals that Sonata opens do not inherit Kimi's session markers, so a Kimi started inside them
behaves as a normal top-level session, not as a nested child of the outer one.

**Why this priority**: Rare, and only matters for developers who run Sonata from an agent. Feature
009 already does the same for Claude Code and Codex.

**Independent Test**: Launch Sonata from an environment that carries Kimi's session markers. Open a
terminal and list its environment: none of those markers are present.

**Acceptance Scenarios**:

1. **Given** Sonata was launched from an environment with Kimi's session markers, **When** any
   terminal starts, **Then** those markers are absent from the terminal's environment.
2. **Given** the user's own Kimi settings in the environment (for example the data folder
   location or API keys), **When** a terminal starts, **Then** those settings are kept.

---

### Edge Cases

- **Kimi run outside Sonata after setup**: the hooks must exit at once without output, so Kimi in a
  normal terminal is unaffected.
- **Sonata uninstalled after setup**: the hooks must not break Kimi; a missing Sonata command makes
  each hook fail quietly. The setup view documents how to remove the lines.
- **User edits Kimi's configuration file by hand**: Sonata detects its lines by content, not by
  position, and never removes or reorders the user's own hooks.
- **Kimi's configuration file is missing or invalid**: setup does not create a broken file; if the
  file cannot be read, Sonata says so and changes nothing.
- **Kimi's data folder moved** (Kimi's own data-folder setting): Sonata looks for the configuration
  in the folder Kimi will use from the terminal's environment.
- **Kimi updated and hook format changed**: Sonata keeps working with the quiet-screen fallback and
  shows the integration as out of date if its lines no longer match.
- **One-shot runs** (`kimi` with a single prompt, non-interactive): do not change the terminal's
  busy state beyond the run itself and do not replace the remembered conversation.
- **Kimi started with a user-chosen conversation or agent file**: Sonata does not add its own resume
  and does not override the user's choice.
- **Typed instruction message opt-in** (feature 003): for Kimi terminals, Sonata's own instruction
  delivery replaces the typed message, so instructions are never given twice.

## Requirements *(mandatory)*

### Functional Requirements

**Preset and launch**

- **FR-001**: The new-terminal dialog MUST offer a "Kimi Code" preset that starts Kimi.
- **FR-002**: Kimi terminals MUST start Kimi through Sonata's agent launcher, as Claude Code and
  Codex do, including when the user types `kimi` by hand in a plain shell terminal.
- **FR-003**: When Kimi is not installed, the terminal MUST show the existing missing-agent error.

**Session instructions**

- **FR-004**: Every new Kimi conversation started in Sonata MUST receive Sonata's session
  instructions and the terminal's role without user action.
- **FR-005**: Delivering instructions MUST NOT prevent resuming a conversation, and MUST NOT add a
  repeated instruction message to a conversation each time it is resumed.
- **FR-006**: Delivering instructions MUST NOT write to the user's project folder or to Kimi's own
  files.

**Integration setup**

- **FR-007**: Sonata MUST detect whether its Kimi integration is on, off, or out of date, by reading
  Kimi's configuration file without changing it.
- **FR-008**: When the integration is off or out of date, Sonata MUST show a dismissible notice on
  Kimi terminals with a Set up action. Dismissal MUST be remembered.
- **FR-009**: The setup view MUST show the exact lines, the target file, and what each line does,
  before anything changes.
- **FR-010**: Sonata MUST change Kimi's configuration file only when the user confirms Apply or
  Remove in the setup view. Before each change it MUST save a backup copy of the file next to it.
  Apply adds only Sonata's block; Remove takes out only Sonata's block.
- **FR-011**: The integration lines MUST keep every existing user setting and hook unchanged, and
  MUST leave Kimi's configuration loadable by Kimi.
- **FR-012**: The integration hooks MUST do nothing, print nothing, and exit successfully when Kimi
  runs outside a Sonata terminal or when Sonata is not running.
- **FR-013**: The setup view MUST explain how to remove the integration.

**Busy and done**

- **FR-014**: With the integration on, Sonata MUST mark a Kimi terminal busy when a message is
  submitted and done when Kimi ends its turn, using Kimi's own signals.
- **FR-015**: With the integration on, a pending ask to Kimi MUST return when Kimi ends its turn,
  not after a quiet period.
- **FR-016**: With the integration off, Kimi terminals MUST use the existing quiet-screen fallback.

**Session restore**

- **FR-017**: With the integration on, Kimi terminals MUST meet feature 009's restore requirements
  (remembering, following switches, resuming, Start fresh, graceful failure, copies start fresh)
  in the same way as Claude Code and Codex terminals.
- **FR-018**: With the integration off, Kimi terminals MUST start fresh on reopen, as today.

**Clean environment**

- **FR-019**: Every terminal Sonata starts MUST NOT inherit Kimi's agent-session markers. The list
  MUST be explicit and limited to session markers; user settings MUST be kept.

### Key Entities

- **Kimi integration status**: computed, not stored. One of on, off, or out of date, derived from
  Kimi's configuration file. Shown on Kimi terminals and in the setup view.
- **Integration notice dismissal**: a per-user preference that the off/out-of-date notice was
  dismissed. Stored with Sonata's preferences; reset when the status changes.
- **Remembered conversation**: the entity from feature 009, with Kimi added as a possible agent.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can go from "no Kimi terminal" to a Kimi agent that answers asks from a linked
  agent in under 1 minute, with no configuration change.
- **SC-002**: A user can set up the Kimi integration in under 1 minute from the notice, on the first
  try.
- **SC-003**: With the integration on, 100% of asks to Kimi return the full answer when Kimi ends its
  turn, including turns with silent periods of 20 seconds or more.
- **SC-004**: With the integration on, after Sonata quits and reopens, 100% of Kimi terminals with a
  remembered conversation resume it, with zero manual steps.
- **SC-005**: After setup, running Kimi outside Sonata shows no visible difference from before setup.
- **SC-006**: Setup never loses or changes a user setting in Kimi's configuration: 0 differences
  outside Sonata's own lines in all tested configurations.

## Assumptions

- "Kimi" means Kimi Code CLI (command `kimi`), the current CLI from Moonshot AI, which replaced the
  older `kimi-cli`. The older CLI is not supported.
- Kimi offers resume by conversation identifier, hooks for session start, prompt submit, and turn
  end, and passes the conversation identifier to every hook, as its public documentation describes.
  Hooks are marked Beta by Kimi; the plan pins the documented behavior and the quickstart lists
  live checks for when Kimi is installed.
- Kimi is not installed on the development Mac. Behavior is verified with fakes and documented
  commands, as in features 006 and 009.
- Feature 009 is merged before this feature's restore work starts. Preset, instructions, setup, and
  busy/done do not depend on 009.
- Kimi's usage limits (feature 006), Kimi's web and IDE modes, and Kimi inside device or browser
  nodes are out of scope.
- The integration is global for the user (one Kimi configuration file), not per workspace or per
  terminal.
