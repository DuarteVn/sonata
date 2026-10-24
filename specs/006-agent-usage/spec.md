# Feature Specification: Agent Usage Meter

**Feature Branch**: `006-agent-usage`

**Created**: 2026-10-23

**Status**: Draft

## Clarifications

### Session 2026-10-23

- Q: Besides plan limit percentages, should the panel show spending per terminal or workspace
  (tokens, dollars)? → A: No. Plan limits only, per account. The user also wants a Usage
  section in Sonata settings (second screenshot): a master switch, one row per agent with its
  own switch, color, detected command path, status and refresh, a "Refresh all" action, and a
  way to add other agents through definition files.

**Input**: User description: "Add a usage view so I can control what Claude and other agents spend" (with a screenshot of a small panel that shows, for Claude Code on a Team plan, the share of the current session limit and of the weekly limit used, each with its reset time, a refresh button, and a settings link).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See how much of my plan limits I used (Priority: P1)

The user runs several agents at once and wants to know, at a glance, how close each agent
account is to its limits. A Usage button in Sonata opens a small panel. For each detected agent
(Claude Code first), the panel shows the account plan name, the share used of the short window
limit (for example, the 5-hour session) and of the weekly limit, and when each one resets.

**Why this priority**: Running many agents in parallel burns limits fast; when a limit runs out,
all agents of that account stop mid-task. Seeing it early lets the user pause or reassign work.

**Independent Test**: With Claude Code logged in on this Mac, open the Usage panel and compare
its percentages and reset times with what Claude Code itself reports for the same account.

**Acceptance Scenarios**:

1. **Given** Claude Code is installed and logged in, **When** the user opens the Usage panel,
   **Then** it shows the plan name, the session percentage, the weekly percentage, and a reset
   time for each, matching Claude Code's own report within 1 percentage point.
2. **Given** the panel is open, **When** the user presses refresh, **Then** the values update
   within 5 seconds, or the panel shows why they could not.
3. **Given** an agent is installed but not logged in, **When** the panel opens, **Then** that
   agent shows "Not logged in" and how to log in, with no percentages.
4. **Given** no supported agent is installed, **When** the panel opens, **Then** it says so and
   lists the agents Sonata can read.
5. **Given** a value is older than 15 minutes because refresh failed, **When** the panel shows
   it, **Then** it marks the value as stale with the time of the last good reading.

---

### User Story 2 - Get warned before a limit runs out (Priority: P2)

The user does not keep the panel open. Sonata shows the highest current usage in a compact form
(for example, a small meter on the Usage button) and sends a macOS notification when an account
crosses a warning level.

**Why this priority**: The panel only helps when the user looks at it. A warning prevents the
surprise stop of every agent on the account.

**Independent Test**: Feed a reading that crosses the warning level and confirm that exactly one
notification appears for that window and that the compact meter changes state.

**Acceptance Scenarios**:

1. **Given** session usage goes from below 80% to 80% or more, **When** Sonata reads it, **Then**
   Sonata sends one notification naming the agent, the window, and the reset time.
2. **Given** a notification was already sent for a window, **When** later readings stay above the
   level, **Then** no repeat notification is sent until that window resets.
3. **Given** usage reaches 100%, **When** Sonata reads it, **Then** the compact meter shows the
   limit as reached and the reset time.
4. **Given** the user turned warnings off in the panel settings, **When** usage crosses a level,
   **Then** no notification is sent, and the compact meter still updates.

---

### User Story 3 - Other agents in the same panel (Priority: P3)

The user also runs Codex (and later other agents). The panel lists each agent that Sonata can
read in its own section, in the same form as Claude Code.

**Why this priority**: The user wants one place for all agent spending; Claude Code covers most
of today's use, so other agents come after it.

**Independent Test**: With Codex installed and used at least once, open the panel and compare its
values with Codex's own status report.

**Acceptance Scenarios**:

1. **Given** Codex is installed and has usage data, **When** the panel opens, **Then** Codex
   shows its limit windows and reset times in its own section.
2. **Given** an agent Sonata cannot read (for example Gemini CLI or OpenCode), **When** the
   panel opens, **Then** that agent is not shown, or is shown as "Usage not available", never
   with invented numbers.

---

### User Story 4 - Control usage reading in Settings (Priority: P2)

The user opens Sonata Settings, section Usage. A master switch turns usage reading on or off for
the whole app. Below it, one row per agent shows the agent's color, whether its command was found
and where, when it was last read (or why not), a switch to turn reading on or off for that agent,
and a refresh action. A "Refresh all" action reads every enabled agent.

**Why this priority**: Reading an agent's login is sensitive; the user must see and control which
agents Sonata reads, and fix problems (command not found, not logged in) from one place.

**Independent Test**: Turn the master switch off and confirm that Sonata reads nothing and hides
the compact meter; turn one agent off and confirm that only that agent stops being read.

**Acceptance Scenarios**:

1. **Given** the master switch is off, **When** time passes or the panel opens, **Then** Sonata
   reads no usage, contacts no provider, and the Usage button shows usage as off.
2. **Given** an agent row is switched off, **When** readings run, **Then** that agent is skipped
   and the panel omits it.
3. **Given** an agent's command is not installed, **When** the user opens Settings, **Then** its
   row says "Command not found" and its switch has no effect until the command appears.
4. **Given** an agent's command is found, **When** the user opens Settings, **Then** the row shows
   the full path of the command and the time since the last good reading.
5. **Given** the user changes an agent's color, **When** the panel and compact meter render,
   **Then** that agent uses the new color.

---

### User Story 5 - Add another agent through a definition file (Priority: P3)

The user runs an agent that Sonata does not know. They put a small definition file in Sonata's
usage providers folder (Settings offers "Show folder" and "Copy path"). The file tells Sonata
which command reports the agent's limits and how to read its answer. A guide file in that folder
explains the format, so the user can ask one of their agents to write the definition.

**Why this priority**: It lets the user cover new agents without a Sonata release, but the
built-in agents cover today's needs.

**Independent Test**: Add a definition for a test command that prints fixed limit values and
confirm that a new row and a new panel section appear with those values.

**Acceptance Scenarios**:

1. **Given** a valid definition file is added, **When** the user presses "Refresh all", **Then**
   the new agent appears in Settings and in the panel.
2. **Given** a definition file is invalid, **When** Sonata loads it, **Then** its row shows the
   error, and other agents are not affected.
3. **Given** a definition is new, **When** Sonata would run its command for the first time,
   **Then** Sonata asks the user to allow it, showing the exact command.

### Edge Cases

- The agent's account has no weekly limit (for example a pay-per-use key): the panel shows only
  the windows that exist, or "No plan limits" for that account.
- Several Claude Code accounts are used on this Mac: Sonata shows the account the agent is
  logged in with now, and names it.
- The usage source changes format or stops answering: the agent shows "Usage unavailable" with
  the reason in the log; the app does not crash and other agents still show.
- The Mac is offline: the panel shows the last good values, marked stale.
- Usage readings run while many agents work: readings never slow terminals, and Sonata reads at
  most once per minute per agent, plus manual refreshes.
- A reset time passes while the panel is open: the panel refreshes that agent.

## Requirements *(mandatory)*

### Functional Requirements

**Panel**

- **FR-001**: Sonata MUST provide a Usage button (with a compact meter) that opens a Usage panel
  without leaving the current workspace.
- **FR-002**: For each supported agent, the panel MUST show: agent name, account plan name when
  known, each limit window (short window and weekly), the percentage used, and the time until
  reset.
- **FR-003**: The panel MUST have a refresh action and MUST show the time of the last reading.
- **FR-004**: The panel MUST show clear states for: not installed, not logged in, usage not
  available, stale data, and limit reached.
- **FR-005**: The compact meter MUST reflect the highest percentage among all windows of all
  agents, and MUST show a distinct state at the warning level and at 100%.

**Readings**

- **FR-006**: Sonata MUST read usage automatically while the app is open, at most once per minute
  per agent, and MUST also read on open of the panel and on refresh.
- **FR-007**: Sonata MUST support Claude Code in the first release. Codex MUST follow when its
  usage can be read; other agents are shown only when a reliable source exists.
- **FR-008**: Sonata MUST NOT display estimated or invented limit values; when a value cannot be
  read, it says so.
- **FR-009**: The panel MUST show plan limits per account only. Per-terminal or per-workspace
  token counts and dollar costs are out of scope. Accounts that pay per use (API keys) have no
  plan limits; the panel says so for them.

**Warnings**

- **FR-010**: Sonata MUST send one macOS notification when a window crosses 80% and one when it
  reaches 100%, at most once per level per window period.
- **FR-011**: Settings MUST let the user turn warnings on or off; the choice persists in
  preferences.

**Settings**

- **FR-015**: Sonata MUST have a Settings window (opened from the app menu and with Cmd+,) with a
  Usage section. This feature adds only the Usage section; other sections come with later
  features.
- **FR-016**: The Usage section MUST offer a master switch, on by default after the first-use
  consent (FR-012), that stops all readings and provider contact when off.
- **FR-017**: The Usage section MUST list each known agent with: name, a built-in or custom tag,
  a color the user can change, status (last reading time, "Command not found", "Not logged in",
  or the error), the detected command path, a per-agent switch, and a refresh action. It MUST
  also offer "Refresh all".
- **FR-018**: Sonata MUST load extra agent definitions from a usage providers folder under
  `~/.sonata/`, and Settings MUST offer "Show folder" and "Copy path" for it. The folder MUST hold
  a guide that explains the definition format.
- **FR-019**: A definition MUST describe a command to run and how to read limit windows from its
  output. Sonata MUST ask the user before running a definition's command the first time, showing
  the exact command, and MUST run it with a time limit of 10 seconds. A definition MUST NOT be
  able to make Sonata read credential stores.

**Privacy and security**

- **FR-012**: Before Sonata reads an agent's stored login to query that agent's provider, it MUST
  ask the user once, per agent, and MUST honor a "No" (that agent's switch in Settings stays off,
  and the user can turn it on later).
- **FR-012a**: Sonata MUST prefer limits the agent's own command reports. It MUST use the
  agent's stored login to ask the provider only when the command does not report limits.
- **FR-013**: Sonata MUST only read agent logins; it MUST NOT change, refresh, copy, store, or log
  them, and MUST send them only to that agent's own provider.
- **FR-014**: Usage values MAY be cached in `~/.sonata/` so the panel shows the last reading after
  a restart; the cache MUST NOT contain credentials.

### Key Entities

- **Agent account**: an agent kind (Claude Code, Codex) plus the account it is logged in with;
  holds the plan name and the reading consent.
- **Limit window**: one limit of an account (short window, weekly); holds the percentage used,
  the reset time, and the last warning level sent in this period.
- **Usage provider**: how Sonata reads one agent kind: built-in or custom (from a definition
  file); holds the command path, the color, and the on/off switch.
- **Usage reading**: a snapshot of all windows of one account at a time, with its source status
  (ok, not logged in, unavailable, stale).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The user sees the usage of every logged-in supported agent in under 2 seconds after
  opening the panel (from cache, then refreshed).
- **SC-002**: Shown percentages match the agent's own report within 1 percentage point, and reset
  times within 1 minute.
- **SC-003**: 100% of window crossings of 80% and 100% produce exactly one notification each.
- **SC-004**: Usage readings add no visible delay to terminals or the canvas while 8 agents run.
- **SC-005**: No credential ever appears in Sonata files, logs, or command lines (verified by a
  search of `~/.sonata/` and the logs after a test session).

## Assumptions

- The user mostly uses Claude Code on a subscription plan (Team) where limits are expressed as a
  share of a short window and a weekly window; the panel follows that model.
- Claude Code and Codex keep usage information that a local app can read for the account that is
  logged in on this Mac. The plan phase confirms the exact sources, with the user's permission
  before any credential store is inspected.
- Some usage sources are not officially documented and may change; FR-008 and the "usage
  unavailable" state keep Sonata honest when they do.
- Clean room: the screenshot shows wanted behavior only. Sonata uses its own layout, wording, and
  visuals; no text, icon, or design is copied from another product.
- Out of scope: changing limits, buying more usage, team-wide dashboards, usage history charts,
  and per-model breakdowns.
