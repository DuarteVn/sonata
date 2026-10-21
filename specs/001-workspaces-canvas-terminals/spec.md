# Feature Specification: Workspaces, Canvas, Terminals, and Notes

**Feature Branch**: `001-workspaces-canvas-terminals`

**Created**: 2026-10-21

**Status**: Draft

**Input**: User description: "Feature 001 — Workspaces, canvas, terminals, and notes (first usable
slice of Sonata). Multiple workspaces with no artificial limit, each an infinite 2D canvas holding
terminal nodes and Markdown note nodes, with durable local persistence."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run terminals on a canvas in one workspace (Priority: P1)

A developer opens Sonata for the first time, creates a workspace for a project folder, and adds
several terminals to the canvas. Each terminal runs a real shell in the project folder, or starts
a chosen command such as an AI coding agent. The developer arranges the terminals on the canvas,
pans and zooms to see them, and works in them as in any terminal app.

**Why this priority**: This is the smallest slice that gives value: several agents side by side
on one surface. Every other story builds on it.

**Independent Test**: Create one workspace, add three terminals (one plain shell, two with a
launch command), type in each, move and resize them, pan and zoom the canvas. Each terminal
behaves like a normal terminal, and layout changes stay in place.

**Acceptance Scenarios**:

1. **Given** no workspaces exist, **When** the user launches the app, **Then** the app shows an
   empty state that offers to create a workspace.
2. **Given** a workspace with working directory `/path/project`, **When** the user adds a
   terminal without a launch command, **Then** an interactive shell starts in `/path/project`.
3. **Given** a workspace, **When** the user adds a terminal with launch command `claude`, **Then**
   the shell starts in the working directory and runs `claude` as its first command.
4. **Given** a terminal on the canvas, **When** the user renames it, moves it, resizes it, or
   brings it to front, **Then** the change is visible at once and is kept after restart.
5. **Given** a running terminal, **When** the user resizes its node, **Then** the program inside
   receives the new size and redraws to fit.
6. **Given** the user pans or zooms the canvas, **When** the user restarts the app, **Then** the
   camera position and zoom of that workspace are the same as before.

---

### User Story 2 - Many workspaces, switch freely (Priority: P1)

A developer works on several projects at once. Each project has its own workspace with its own
canvas, terminals, and notes. The developer creates as many workspaces as needed, sees them all
in a sidebar, and switches between them. Agents keep working in the workspaces that are not on
screen.

**Why this priority**: Unlimited workspaces are the reason Sonata exists.

**Independent Test**: Create five workspaces for five different folders, start a long-running
command in a terminal of workspace A, switch to B, C, and back to A. The command kept running and
its output is complete.

**Acceptance Scenarios**:

1. **Given** any number of existing workspaces, **When** the user creates another one with a name
   and an existing folder, **Then** it is created; the app never refuses because of a count limit.
2. **Given** a new workspace is created, **Then** it gets a random icon and a color, preferring a
   color that no other workspace uses while unused colors remain.
3. **Given** the user enters a folder that does not exist, or a name that is empty or already used
   (ignoring case and surrounding spaces), **When** the user confirms, **Then** the app refuses
   with a message that names the problem, and nothing is created.
4. **Given** several workspaces, **When** the user renames, pins, or reorders them in the sidebar,
   **Then** pinned workspaces appear first and the chosen order is kept after restart.
5. **Given** a terminal running in workspace A, **When** the user switches to workspace B,
   **Then** the terminal in A keeps running and its output is not lost.
6. **Given** the user chooses to delete a workspace, **When** the user confirms the deletion
   dialog, **Then** its terminals stop, the workspace and its Sonata data are removed, and the
   project folder on disk is not touched. **When** the user cancels, **Then** nothing changes.

---

### User Story 3 - Work survives restarts and crashes (Priority: P1)

A developer quits the app, or the app crashes, or the computer loses power. When the app starts
again, every workspace comes back with its layout, notes, and the history of each terminal.

**Why this priority**: A layout of many agents is costly to rebuild. Losing it once destroys
trust in the tool.

**Independent Test**: Arrange two workspaces, write notes, run commands, then force-kill the app.
Restart: layouts and notes are back with at most the last autosave interval of changes lost, and
each terminal shows its earlier output as history.

**Acceptance Scenarios**:

1. **Given** the user changed a workspace, **When** the autosave interval (default 30 seconds)
   passes or the user quits normally, **Then** the change is saved.
2. **Given** no workspace changed, **When** the autosave interval passes, **Then** no file is
   rewritten.
3. **Given** the app is killed during a save, **When** it starts again, **Then** each workspace
   file holds either the previous complete version or the new complete version, never a mix.
4. **Given** the app restarts, **When** a workspace opens, **Then** each terminal shows its saved
   output as read-only history above a fresh session, according to FR-016.
5. **Given** one workspace file is unreadable, **When** the app starts, **Then** all other
   workspaces load normally, and the broken one appears in the sidebar as a load failure with
   the reason and the file location. The broken file is not overwritten.
6. **Given** the app crashed shortly after launch two times in a row, **When** it starts a third
   time, **Then** it does not reopen the last active workspace automatically and tells the user
   why. The user can still open any workspace by hand.
7. **Given** the user restarts the app, **Then** it reopens the workspace that was active and the
   sidebar shows recently used workspaces.

---

### User Story 4 - Markdown notes beside the terminals (Priority: P2)

A developer keeps instructions, plans, and to-do lists as sticky notes on the canvas next to the
agents that use them.

**Why this priority**: Useful, but terminals and workspaces deliver value without it.

**Independent Test**: Add a note, write Markdown in it, view the rendered result, restart the app.
The note is unchanged and also exists as a readable `.md` file on disk.

**Acceptance Scenarios**:

1. **Given** a workspace, **When** the user adds a note, **Then** an empty note appears at the
   chosen spot on the canvas and is ready for typing.
2. **Given** a note, **When** the user edits it, **Then** the user writes Markdown text, and when
   editing ends the note shows the rendered Markdown.
3. **Given** a saved note, **When** the user opens the workspace's notes folder on disk, **Then**
   the note exists as a plain `.md` file with the same text.
4. **Given** a note, **When** the user moves, resizes, or deletes it, **Then** the canvas and the
   files on disk reflect the change after the next save.

---

### User Story 5 - Many terminals stay fast (Priority: P2)

A developer places dozens of terminals on one canvas, zooms out to see the whole picture, and
zooms in to work in one of them. The app stays responsive the whole time.

**Why this priority**: Required for heavy use, but a small canvas works without it.

**Independent Test**: Open 30 terminals in one workspace, each printing output continuously. Pan
and zoom across the canvas, then type in one terminal. Panning stays smooth, typing echoes at
once, and every terminal still shows correct, complete output when brought into view.

**Acceptance Scenarios**:

1. **Given** terminals that are off screen or too small to read, **Then** the app shows them as a
   lightweight preview, and their programs keep running.
2. **Given** a terminal shown as a preview, **When** the user zooms or pans until it is legible,
   **Then** it becomes fully interactive and shows its current screen with no lost output.
3. **Given** the app hides a terminal view for any reason, **Then** the program in the terminal
   keeps running. Only an explicit "close terminal" or workspace deletion stops it.

---

### Edge Cases

- The working directory of a workspace is deleted or renamed after creation: terminals that start
  later fall back to the user's home folder, and the workspace shows a warning with the missing
  path.
- A launch command does not exist: the shell shows the normal "command not found" error and stays
  open.
- The program in a terminal exits: the node stays on the canvas with an "exited" state and the
  exit code, and offers a restart.
- The user closes a terminal that runs a program other than the idle shell: the app asks for
  confirmation first.
- The disk is full or the data folder is not writable during save: the app keeps the data in
  memory, shows a persistent warning, and retries at the next autosave. The previous files remain
  intact.
- A workspace file has a newer format version than this app supports: it is shown as a load
  failure ("created by a newer version") and is never overwritten.
- Two workspaces point to the same folder: allowed. Names stay unique.
- Terminal output is very large: saved history is capped (see Assumptions), and the oldest lines
  are dropped first.
- The user zooms far in or out: zoom stays inside fixed bounds, so the canvas never becomes empty
  or unreadable by accident.

## Requirements *(mandatory)*

### Functional Requirements

**Workspaces**

- **FR-001**: Users MUST be able to create a workspace with a name and an existing local folder
  as its working directory. The app MUST NOT limit the number of workspaces.
- **FR-002**: The app MUST reject an empty name, a name equal to an existing workspace name
  (ignoring case and surrounding spaces), and a folder that does not exist, each with a specific
  message.
- **FR-003**: Each new workspace MUST get a randomly chosen icon and color. The color MUST be one
  not used by another workspace while such a color is available.
- **FR-004**: Users MUST be able to rename, pin, unpin, and reorder workspaces in a flat sidebar
  list. Pinned workspaces appear before unpinned ones.
- **FR-005**: Users MUST be able to switch the active workspace from the sidebar. Switching MUST
  NOT stop terminals in other workspaces.
- **FR-006**: Users MUST be able to delete a workspace only after confirming a dialog that names
  it. Deletion stops its terminals and removes its Sonata data. Deletion MUST NOT modify the
  workspace's working directory.

**Canvas**

- **FR-007**: Each workspace MUST have its own unbounded 2D canvas with pan and zoom inside fixed
  zoom bounds.
- **FR-008**: Users MUST be able to add, select, move, resize, delete, and bring to front terminal
  and note nodes.
- **FR-009**: The app MUST save, per workspace, every node's position, size, and stacking order,
  and the camera position and zoom.

**Terminals**

- **FR-010**: A terminal node MUST run a real interactive shell, by default the user's login
  shell, started in the workspace working directory unless the node sets another folder.
- **FR-011**: A terminal node MAY have a launch command that runs as the first command of the
  shell when the terminal starts.
- **FR-012**: Users MUST be able to set and edit a terminal's name. The app MUST NOT limit the
  number of terminals in a workspace.
- **FR-013**: Terminal processes MUST keep running when their view is off screen, shown as a
  preview, or in an inactive workspace. Only closing the terminal, deleting the workspace, or
  quitting the app stops them.
- **FR-014**: Terminals that are not legible on screen MUST be shown as a lightweight preview, and
  MUST become fully interactive with their current screen when they become legible.
- **FR-015**: The app MUST save each terminal's output history to disk, capped in size, and show
  it as read-only history when the workspace opens after a restart.
- **FR-016**: After a restart, when a workspace opens, each terminal MUST start a fresh shell
  below its saved history and, if it has a launch command, MUST run that command again. The
  program starts a new session; its earlier in-memory state is not restored.
- **FR-017**: A terminal whose program exits MUST show an exited state with the exit code and
  offer a restart.
- **FR-018**: Closing a terminal that runs a program other than its idle shell MUST require
  confirmation.

**Notes**

- **FR-019**: Users MUST be able to add Markdown notes to the canvas, edit their text in place,
  and see the rendered result when not editing.
- **FR-020**: Each note's text MUST be stored as a separate plain `.md` file inside that
  workspace's data folder.

**Persistence and recovery**

- **FR-021**: All app data MUST live in human-readable files inside `~/.sonata/`, with one folder
  per workspace.
- **FR-022**: Every data file MUST carry its type and a format version, and the app MUST upgrade
  older formats to the current one when reading.
- **FR-023**: Saving MUST be all-or-nothing per file: an interrupted save leaves the previous
  complete file in place.
- **FR-024**: The app MUST save changed workspaces automatically at a configurable interval
  (default 30 seconds) and on normal quit, and MUST NOT rewrite unchanged workspaces.
- **FR-025**: At launch, the app MUST find workspaces by scanning the workspaces folder. A
  workspace that fails to load MUST appear in the sidebar as a load failure with its reason and
  file path, MUST NOT stop the rest from loading, and MUST NOT be overwritten.
- **FR-026**: The app MUST remember the active workspace and the recently used workspaces across
  restarts.
- **FR-027**: After two consecutive crashes shortly after launch, the app MUST NOT reopen the last
  active workspace automatically, and MUST tell the user why. A launch that stays alive past the
  early-crash window resets the count.

### Key Entities

- **Workspace**: a named project space. Has an id, name, icon, color, working directory, pinned
  flag, sidebar position, creation and last-opened times, camera position and zoom, and a set of
  nodes.
- **Node**: an item on a workspace's canvas. Has an id, position, size, stacking order, creation
  and modification times, and exactly one kind of content: terminal or note.
- **Terminal (node content)**: name, optional launch command, optional working folder, run state
  (starting, running, exited with code, stopped), and a reference to its saved output history.
- **Note (node content)**: a reference to its `.md` file.
- **App state**: active workspace, recent workspaces, sidebar order, clean-shutdown marker, and
  early-crash counter.
- **Preferences**: user settings, including the autosave interval.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A new user creates a first workspace and has a running terminal in it in under
  1 minute without help.
- **SC-002**: A user can create at least 50 workspaces, and switching between any two takes under
  1 second.
- **SC-003**: With 30 terminals printing output continuously in one workspace, panning and zooming
  stay visually smooth, and keystrokes in the focused terminal echo with no noticeable delay.
- **SC-004**: With 100 terminals running across all workspaces, every terminal brought into view
  shows complete, correct output.
- **SC-005**: In 100 forced kills of the app during active use, no data file is ever left corrupt,
  and at most the last autosave interval of layout and note changes is lost.
- **SC-006**: A single corrupt workspace file never prevents the other workspaces from loading
  (verified with deliberately broken files).
- **SC-007**: Users can read and edit every note with any text editor, without the app.

## Assumptions

- Users are developers on macOS who run several AI coding agents in parallel across several
  projects. Sonata runs commands with the user's own permissions.
- Running programs do not survive app quit; only layout, notes, and terminal output history do.
- Terminals of a workspace start when that workspace is first opened in a session, not at app
  launch, so that idle workspaces cost nothing.
- Saved terminal history is capped at 10,000 lines per terminal.
- The early-crash window is 30 seconds after launch.
- Zoom bounds are 10% to 400%.
- Deleting a workspace is permanent in this feature; backups come in a later feature.
- Only local folders are supported. Remote machines, containers, links between nodes,
  agent-to-agent commands, orchestration roles, git isolation, templates, cloning, backups,
  browser nodes, sidebar groups and folders, and routines are out of scope.
- The practical limit on terminals is the computer's memory and CPU, not the app.
