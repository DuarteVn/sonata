# Feature Specification: Canvas Toolbars, New Terminal Dialog, and Visual Links

**Feature Branch**: `002-canvas-toolbars-links`

**Created**: 2026-10-22

**Status**: Draft

**Input**: User description: "Feature 002 — Canvas toolbars, New Terminal dialog, and visual links.
A bottom tool bar (Select, Terminal, Note), a New Terminal dialog with quick-start presets and
Details/Appearance/Role tabs, floating toolbars for a selected note or terminal, and visual links
between nodes that are saved with the workspace. Links become the permission for agents to talk to
each other in feature 003."

Builds on feature 001 (workspaces, canvas, terminals, Markdown notes). Sonata uses its own visual
design and wording.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start an agent in two clicks (Priority: P1)

A developer wants a new Claude Code agent in the current project. They pick the Terminal tool in
the bottom tool bar, click where the terminal should go, choose the "Claude Code" preset, and
confirm. The terminal appears at that spot, already running the agent.

**Why this priority**: Starting agents is the most frequent action in Sonata. Today it needs typing
a command by hand.

**Independent Test**: With an open workspace, start one terminal from each preset and one plain
shell, using only the tool bar and the dialog. Each starts the right program in the right folder
at the clicked spot.

**Acceptance Scenarios**:

1. **Given** an open workspace, **When** the user looks at the canvas, **Then** a tool bar at the
   bottom shows the Select, Terminal, and Note tools, with Select active.
2. **Given** the Terminal tool is active, **When** the user clicks on the canvas, **Then** the New
   Terminal dialog opens, and the created terminal is placed with its top-left corner at the
   clicked point.
3. **Given** the user activates the Terminal tool with its keyboard shortcut, **When** the dialog
   opens without a canvas click, **Then** the terminal is placed at the center of the visible
   canvas.
4. **Given** the dialog is open, **When** the user picks the "Codex" preset, **Then** the name and
   command fields are filled for Codex, and the user can still edit them before creating.
5. **Given** a preset whose program is not installed, **Then** the preset is marked as not found
   with a short hint, and choosing it still fills the fields.
6. **Given** the dialog, **When** the user leaves the command empty, **Then** the terminal runs a
   plain shell.
7. **Given** the dialog, **When** the user changes the working folder with Browse…, **Then** the
   terminal starts in that folder.
8. **Given** the user created a terminal, **Then** the active tool returns to Select.

---

### User Story 2 - Link nodes on the canvas (Priority: P1)

A developer arranges an orchestrating agent, two worker agents, and a plan note. They draw links
from the orchestrator to each worker and to the note, so the canvas shows who works with whom.
Links stay in place when nodes move and come back after a restart.

**Why this priority**: Links are the base of agent collaboration in feature 003 and the user asked
for them first.

**Independent Test**: Create three terminals and a note, link them in a star, move nodes, delete
one link and one node, restart. The remaining links are drawn correctly and the counts are right.

**Acceptance Scenarios**:

1. **Given** two nodes, **When** the user drags from the link handle of one node and releases over
   the other, **Then** a link is drawn between them as a curve.
2. **Given** a selected node, **When** the user picks the link action in its toolbar and then
   clicks another node, **Then** a link is created between them. Escape or a click on empty canvas
   cancels.
3. **Given** two nodes already linked, **When** the user tries to link them again in either
   direction, **Then** no second link is created.
4. **Given** a drag that ends on the same node or on empty canvas, **Then** no link is created.
5. **Given** linked nodes, **When** the user moves or resizes one, **Then** the link follows during
   the move.
6. **Given** a link, **When** the user clicks it and presses Delete, or uses its delete control,
   **Then** the link is removed.
7. **Given** a node with links, **When** the node is deleted, **Then** all its links are removed.
8. **Given** links exist, **When** the app restarts, **Then** the same links are drawn.
9. **Given** any node, **Then** it shows its number of links; the number updates at once when
   links change.

---

### User Story 3 - Configure and edit a terminal (Priority: P2)

A developer marks one agent as the Orchestrator, gives it a role ("You coordinate the workers;
never edit files yourself"), and colors it purple. Later they change its font size and role from
the terminal's edit action. They save the role to a library and reuse it for another terminal.

**Why this priority**: Makes many agents distinguishable and prepares orchestration (feature 003),
but terminals work without it.

**Independent Test**: Create a terminal with every option set, restart the app, open the edit
action: every value is kept. Save a role to the library, create another terminal, and pick that
role.

**Acceptance Scenarios**:

1. **Given** the dialog, **Then** it has three tabs: Details, Appearance, and Role.
2. **Given** the Details tab, **Then** it offers name, command, working folder, "Monitor activity"
   (on by default), and "Orchestrator" (off by default), each with a short explanation.
3. **Given** the Appearance tab, **When** the user picks an accent color and a font size, **Then**
   the terminal node uses that color and its text uses that size.
4. **Given** the Role tab, **When** the user writes a role and chooses "Save to library" with a
   name, **Then** the role appears in the role list of every New Terminal dialog after that,
   including after restart.
5. **Given** a terminal with a role or the Orchestrator option, **Then** its node shows a visible
   marker for each (role name or first words, and an Orchestrator badge).
6. **Given** a terminal, **When** the user picks Edit in its toolbar, **Then** the same dialog opens
   with the current values; saving applies name, color, font size, role, and options at once.
7. **Given** the user edits the command or working folder of a running terminal, **Then** the change
   applies at the next start or restart, and the dialog says so.
8. **Given** "Monitor activity" is on, **Then** the node shows whether the terminal is running a
   program, idle, or exited; **When** a busy terminal becomes idle while it is not focused, **Then**
   the node shows an attention badge until the user focuses it.

---

### User Story 4 - Format notes from a toolbar (Priority: P2)

A developer selects a note and uses a floating toolbar to color it, make the text bigger, add a
checklist, bold a word, and paste in a screenshot, without typing Markdown syntax by hand.

**Why this priority**: Notes work today with raw Markdown; the toolbar makes them faster to use.

**Independent Test**: Select a note, apply every toolbar action once, restart. The note keeps its
color, size, text, and image, and its `.md` file contains standard Markdown.

**Acceptance Scenarios**:

1. **Given** a selected note, **Then** a toolbar floats above it with: color, text smaller, text
   larger, bold, italic, strikethrough, inline code, heading, checklist, bullet list, code block,
   insert image, rendered/raw toggle, link action with link count, and delete.
2. **Given** text is selected in the note editor, **When** the user picks bold, **Then** the
   selection is wrapped in bold Markdown; picking it again on the same selection removes it.
3. **Given** the cursor is on a line, **When** the user picks heading, checklist, or bullet list,
   **Then** the line gets the matching Markdown prefix (heading cycles level 1→2→3→none).
4. **Given** no text is selected, **When** the user picks bold, italic, strikethrough, inline code,
   or code block, **Then** empty markers are inserted with the cursor between them.
5. **Given** the note is not being edited, **When** the user picks a formatting action, **Then** the
   note enters editing and the action applies at the end of the text.
6. **Given** a selected note, **When** the user picks insert image and chooses a picture file,
   **Then** the picture is copied into the workspace's data folder, a Markdown image reference is
   inserted, and the rendered note shows the picture.
7. **Given** a note in rendered mode, **When** the user toggles raw, **Then** the plain Markdown is
   shown (read-only until edited); toggling again shows the rendered view.
8. **Given** a rendered note with a checklist, **When** the user clicks a checkbox, **Then** the item
   is checked or unchecked in the note text.
9. **Given** the user picks a color or text size, **Then** the note keeps it after restart.

---

### User Story 5 - Act on a selected terminal (Priority: P3)

A developer selects a terminal and uses its floating toolbar to edit it, link it, restart it, or
delete it.

**Why this priority**: Convenience over actions that exist elsewhere (title bar, keyboard).

**Independent Test**: Select a terminal and use each toolbar action once.

**Acceptance Scenarios**:

1. **Given** a selected terminal, **Then** a toolbar floats above it with: edit, link action with
   link count, restart, and delete.
2. **Given** restart on a running terminal, **Then** the user confirms when a program is running,
   and a fresh shell starts with the terminal's command.
3. **Given** delete, **Then** the existing confirmation applies when a program is running.

---

### Edge Cases

- A preset's program is installed in a folder that only the user's login shell adds to `PATH`:
  detection uses the login shell's `PATH`, so it is found.
- The tool bar and floating toolbars would cover each other or leave the window: the floating
  toolbar flips below the node or clamps to the visible area.
- A node is far off screen when selected from a link: the toolbar is not shown until the node is
  visible.
- Zoomed far out: link curves stay at least 1 screen pixel wide and the count badge stays readable
  or hides below a minimum zoom.
- An image file is very large: images over 20 MB are refused with a message.
- An image is removed from the data folder outside the app: the note shows a broken-image marker,
  and the text is unchanged.
- A role in the library is deleted: terminals that used it keep their own copy of the text.
- The Note tool is active and the user clicks on an existing node: the click selects that node and
  the tool stays active; only a click on empty canvas places a note.
- Delete pressed while a terminal or text field has keyboard focus: the key goes to that terminal
  or field and nothing is deleted.
- A link connects nodes of which one fails to load later (corrupt workspace is all-or-nothing, so
  this cannot happen inside one workspace); links to unknown node ids found in a file are dropped
  on load with a warning.

## Requirements *(mandatory)*

### Functional Requirements

**Tool bar and selection**

- **FR-001**: Each open canvas MUST show a tool bar at the bottom center with the tools Select,
  Terminal, and Note, and MUST show which tool is active. Select is active by default.
- **FR-002**: Each tool MUST have a single-key shortcut that works when no terminal or text field
  has keyboard focus (V = Select, T = Terminal, N = Note) and a tooltip naming the tool and its key.
- **FR-003**: With the Terminal tool, a click on empty canvas MUST open the New Terminal dialog and
  place the created terminal at the clicked point; activating it by shortcut or tool bar click
  without a canvas click places the terminal at the viewport center. With the Note tool, a click on
  empty canvas MUST create a note at that point. After creating, the active tool returns to Select.
- **FR-004**: A click on a node MUST select it; a click on empty canvas, or Escape when no terminal
  or text field has keyboard focus, MUST clear the selection. Only one node or one link is selected
  at a time.
- **FR-005**: Delete or Backspace MUST delete the selected node or link when no terminal or text
  field has keyboard focus, with the same confirmations as the other delete actions.

- **FR-028**: Cmd+W MUST close the selected terminal or note (or, with nothing selected, the
  focused terminal), with the same confirmation as other deletes; it MUST NOT close the window.
  Cmd+Q keeps quitting the app (001 FR-018a). Added 2026-10-22 from user feedback.
- **FR-029**: With the Select tool, dragging on empty canvas MUST draw a selection rectangle that
  selects every node it touches; Shift+click on a node, or Shift+drag, MUST add to or remove from
  the selection. Dragging the title bar of a selected node MUST move all selected nodes; Delete,
  Backspace, and Cmd+W MUST delete all of them, asking once for terminals still running a
  program. Floating toolbars and link handles show only for a single selection. Added
  2026-10-22 from user feedback (supersedes "one at a time" in FR-004 and the multi-select
  exclusion below).
- **FR-030**: In a live terminal, Shift+Enter MUST send Escape followed by Return, which agent
  CLIs read as a new line, and Option MUST type characters (for example Option+C types "ç")
  instead of acting as Meta. Added 2026-10-22 from user feedback.

**New Terminal dialog**

- **FR-006**: The dialog MUST offer the presets Claude Code, Codex, OpenCode, Gemini CLI, and
  Shell. Picking a preset fills the name and command fields; Shell leaves the command empty.
- **FR-007**: The dialog MUST mark presets whose program is not found in the user's login-shell
  `PATH` as not found, with a hint, and MUST still let the user pick them.
- **FR-008**: The Details tab MUST offer name, command (empty means a plain shell), working folder
  (default: the workspace folder; Browse… to choose another existing folder), "Monitor activity"
  (default on), and "Orchestrator" (default off). The Orchestrator option is stored and shown in
  this feature; its powers are defined in feature 003.
- **FR-009**: The Appearance tab MUST offer an accent color from a palette of at least 8 colors
  (default: none) and a font size override from 9 to 24 (default: the app preference).
- **FR-010**: The Role tab MUST offer a role text area, a list of saved roles to pick from, "Save to
  library" (asks for a role name, unique ignoring case), and deleting roles from the library. The
  terminal stores its own copy of the role text.
- **FR-011**: The same dialog MUST open from a terminal's Edit action with its current values.
  Name, appearance, role, and options apply at once; command and working folder apply at the next
  start or restart, and the dialog MUST state this when they changed for a running terminal.

**Terminal node display**

- **FR-012**: A terminal node MUST show its accent color, an Orchestrator badge when set, and a
  role marker (role name if picked from the library, else the first words of the role) when set.
- **FR-013**: With "Monitor activity" on, a terminal node MUST show its state (running a program,
  idle, exited) and an attention badge when a busy terminal becomes idle while not focused. The
  badge clears when the terminal gets focus. With it off, none of these are shown.

**Floating toolbars**

- **FR-014**: A selected node MUST show a floating toolbar above it (below it if there is no room),
  inside the visible canvas, following the node when it moves or the camera changes.
- **FR-015**: The terminal toolbar MUST offer: edit, link action with link count, restart (with
  confirmation when a program is running), and delete.
- **FR-016**: The note toolbar MUST offer: color (palette of at least 6 note colors), text smaller,
  text larger (at least 5 sizes), bold, italic, strikethrough, inline code, heading, checklist,
  bullet list, code block, insert image, rendered/raw toggle, link action with link count, and
  delete.
- **FR-017**: Formatting actions MUST produce standard Markdown (CommonMark plus GitHub task lists
  and strikethrough) in the note's text, following the wrap/toggle/prefix rules of User Story 4.
- **FR-018**: Insert image MUST copy the chosen picture (PNG, JPEG, GIF, WebP, at most 20 MB) into
  the workspace's data folder under a new unique name, insert a Markdown image reference relative to
  the note, and display it in the rendered note. The original file is not changed.
- **FR-019**: Clicking a checkbox in a rendered note MUST toggle that item in the note text.
- **FR-020**: Note color and text size MUST be saved with the note's node.

**Links**

- **FR-021**: Users MUST be able to create a link between two different nodes by dragging from a
  node's link handle onto another node, or by the toolbar link action followed by a click on the
  target node.
- **FR-022**: A link MUST be undirected and unique per pair of nodes; self-links MUST be refused.
- **FR-023**: Links MUST be drawn as curves between the nodes' edges, updated live while nodes move
  or resize, and drawn beneath nodes.
- **FR-024**: Users MUST be able to select a link (click on it) and delete it.
- **FR-025**: Deleting a node MUST delete its links.
- **FR-026**: Links MUST be saved with the workspace and restored on load. Links that point to
  unknown nodes MUST be dropped on load with a warning.
- **FR-027**: Every node MUST show its link count when it has at least one link.

### Key Entities

- **Link**: a connection between two nodes of the same workspace. Has an id, the two node ids
  (unordered), and a creation time. Unique per pair.
- **Terminal settings (extends Terminal from 001)**: accent color (optional), font size override
  (optional), role text (optional), role name (optional, when picked from the library),
  monitor-activity flag, orchestrator flag.
- **Note style (extends Note from 001)**: color, text size step.
- **Role (library entry)**: a named, reusable instruction text, stored once for the user, available
  in every workspace.
- **Preset**: built-in quick-start entry with a display name, default terminal name, and command.
- **Note image**: a picture file copied into the workspace's data folder and referenced from a
  note's Markdown.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user starts a Claude Code terminal at a chosen spot in at most 3 clicks and no
  typing.
- **SC-002**: A user links two nodes in one drag or two clicks, and the link appears without
  visible delay.
- **SC-003**: With 30 nodes and 40 links on one canvas, moving a node keeps its links visually
  attached with no visible lag.
- **SC-004**: After a restart, 100% of links, terminal settings, note styles, roles, and note images
  are restored.
- **SC-005**: Every note toolbar action produces Markdown that renders the same in another Markdown
  viewer (checked against a reference set of samples).
- **SC-006**: First-time users find and use the link action without instructions (checked with at
  least 2 friends of the author).

## Assumptions

- Preset commands: Claude Code `claude`, Codex `codex`, OpenCode `opencode`, Gemini CLI `gemini`,
  Shell (none). Program detection runs once per dialog opening with a time limit of 2 seconds;
  if it times out, presets are shown without the not-found mark.
- Shortcuts are single keys (V, T, N, Delete, Escape) because the canvas has no text focus when
  they apply; this matches common canvas tools and does not clash with terminals, which keep their
  keys while focused.
- Note colors are a fixed palette of 6 (yellow default, plus 5 others); text sizes are 5 steps
  (default is step 3, the 001 size).
- Terminal accent palette reuses the 8 workspace colors.
- Note images live in `files/` inside the workspace's data folder; deleting a note does not delete
  images, because other notes may reference them.
- The role library is a file in the data folder, shared by all workspaces, private to the user
  like every other Sonata file.
- Moving a link endpoint to another node is out of scope; delete and recreate instead.
- Out of scope: CLI, agent-to-agent messaging, orchestrator powers, delivering roles to agents,
  chat view of terminals, files, folder tree, browser, free text, drawing tools, and multi-select.
