# Feature Specification: Browser Node on the Canvas, Controllable by Linked Agents

**Feature Branch**: `004-browser-node`

**Created**: 2026-10-22

**Status**: Draft

**Input**: User description: "Feature 004 — Browser node on the canvas, controllable by linked
agents."

Builds on 001–003 (canvas, nodes, links, the `sonata` command with link-based permissions).

## Clarifications

### Session 2026-10-22 (decided before specification)

- Q: What must the browser do? → A: The user browses in it, and terminals linked to it can
  control it (open, read, click, fill, screenshot).
- Q: How are cookies and logins shared? → A: Shared by browser nodes of the same workspace,
  isolated between workspaces.
- Q: The page cannot scale with the canvas and would cover menus. How to handle it? → A: The page
  is live only at a legible zoom and when nothing would be covered; otherwise the node shows a
  card with title, URL, and the last captured image of the page.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Browse next to the agents (Priority: P1)

A developer runs a dev server in a terminal and places a browser node beside it, pointed at
`localhost:3000`. They click around the app, reload after changes, and open the developer tools,
all on the canvas next to the agents doing the work.

**Why this priority**: Seeing the result of agents' work without switching windows is the base of
the feature; agent control builds on it.

**Independent Test**: Place a browser node, open a local page and a public site, navigate back and
forward, reload, resize the node, restart the app: the node comes back on the same page.

**Acceptance Scenarios**:

1. **Given** an open workspace, **When** the user picks the Browser tool (B) and clicks the canvas,
   **Then** a browser node of 1024×720 appears at that point with a start page and the address
   field focused.
2. **Given** the address field, **When** the user types `localhost:3000` and presses Enter,
   **Then** the node loads `http://localhost:3000`; typing `example.com` loads
   `https://example.com`; typing words with spaces searches nothing and shows "Enter an address".
3. **Given** a loaded page, **Then** the node shows the page title, a loading indicator while
   loading, and working back, forward, reload, and stop buttons.
4. **Given** a page, **When** the user picks "Open in default browser", **Then** the system browser
   opens the same URL.
5. **Given** a page, **When** the user picks "Inspect", **Then** the page's developer tools open.
6. **Given** the node, **Then** it can be moved, resized (the page reflows to the new size),
   linked, selected, and deleted like other nodes, and its floating toolbar offers link, reload,
   open externally, and delete.
7. **Given** a browser node, **When** the app restarts, **Then** the node reopens its last URL.

---

### User Story 2 - The page stays out of the way (Priority: P1)

The developer pans and zooms around a canvas with several browsers. Pages never float detached
from their nodes, never cover menus or dialogs, and far-away browsers cost nothing.

**Why this priority**: Without it the browser node breaks the canvas experience.

**Independent Test**: With three browser nodes, pan, zoom out and in, open a dialog and a toolbar
over a browser, open six browsers: no page is ever drawn outside its node or above a dialog, and
at most four are live at once.

**Acceptance Scenarios**:

1. **Given** a browser node at a legible zoom, nothing covering it, and within the live limit,
   **Then** its page is live and interactive.
2. **Given** a pan or zoom gesture in progress, **Then** every browser node shows its card (title,
   URL, last image of the page) until the gesture ends, and live pages return within 0.3 seconds
   after it ends.
3. **Given** the zoom is below the legible level, **Then** browser nodes show cards.
4. **Given** a dialog, menu, floating toolbar, or banner would overlap a live page, **Then** that
   page shows its card while the overlap lasts.
5. **Given** more than 4 browser nodes would be live, **Then** only the 4 largest on screen are
   live (the selected one first) and the others show cards.
6. **Given** a browser node shows its card, **Then** its page stays loaded (scripts keep running),
   so it returns to the same state when live again.

---

### User Story 3 - Agents test the UI through the browser (Priority: P2)

A developer links a Claude Code terminal to a browser node and asks it to "check that the login
form shows an error for a wrong password". The agent opens the page, reads it, fills the form,
clicks the button, reads the result, and takes a screenshot to confirm.

**Why this priority**: Lets agents verify their own work in a real browser; depends on US1.

**Independent Test**: From a linked shell, run each `sonata browser` command against a local test
page and check the results; from an unlinked shell, every command is refused.

**Acceptance Scenarios**:

1. **Given** a terminal linked to browser "App", **When** it runs `sonata browser open App
   localhost:3000`, **Then** the browser navigates, the command waits until the page finished
   loading (at most 30 seconds by default), and prints the final URL and title; a failed
   navigation prints the error.
2. **Given** a loaded page, **When** the terminal runs `sonata browser read App`, **Then** it prints
   the title, URL, the readable text of the page, and a numbered list of interactive elements
   (links, buttons, inputs, selects) with their labels.
3. **Given** the numbered list, **When** the terminal runs `sonata browser click App 7` or with a
   CSS selector, **Then** that element is clicked, the command waits for any navigation or
   network-quiet moment (at most 10 seconds), and prints the resulting URL.
4. **Given** an input, **When** the terminal runs `sonata browser fill App 3 "secret"`, **Then** the
   field contains that text, and the page receives the same input events as from typing.
5. **Given** a page, **When** the terminal runs `sonata browser press App Enter`, **Then** the key
   is sent to the focused element.
6. **Given** a page, **When** the terminal runs `sonata browser screenshot App`, **Then** an image of
   the visible page is saved and its path printed; `--out file.png` saves to that path.
7. **Given** `sonata browser back|forward|reload App`, **Then** the browser does it and the command
   waits for the load.
8. **Given** an Orchestrator terminal linked to the browser, **When** it runs `sonata browser eval
   App "document.title"`, **Then** it prints the result as JSON; a non-orchestrator is refused.
9. **Given** a terminal not linked to the browser, **Then** every browser command is refused and
   lists what it is linked to.
10. **Given** the browser node shows its card (hidden), **Then** agent commands still work, and the
    screenshot shows the page as it would appear live.

---

### Edge Cases

- The address is invalid or the server is down: the node shows an error page with the reason
  and a retry button; `sonata browser open` exits with the error.
- A page opens a popup or a link with a new-window target: it opens in the same node.
- A page asks for camera, microphone, or location: denied, with a note in the node.
- File downloads: handed to the system browser with a note (no download manager).
- A page tries to navigate to a non-web scheme (`file:`, app schemes): blocked, except `http`,
  `https`, and `about:blank`.
- The page is very slow: commands time out with a clear message and the node keeps loading.
- An element number from an earlier `read` no longer exists after the page changed: the command
  fails and says to run `read` again.
- Basic-auth or certificate errors: shown as an error card; certificate errors are never ignored.
- The workspace is deleted: its browsing data (cookies, storage) is deleted too.

## Requirements *(mandatory)*

### Functional Requirements

**Node and navigation**

- **FR-001**: The tool bar MUST offer a Browser tool (shortcut B) that places a 1024×720 browser
  node at the clicked point.
- **FR-002**: The address field MUST accept a URL or host; text without a scheme becomes `https://`,
  except `localhost`, `127.0.0.1`, and `[::1]` (with optional port and path), which become
  `http://`. Text with spaces is refused with a hint.
- **FR-003**: The node MUST show title, loading state, and back, forward, reload/stop, open in
  default browser, and inspect controls.
- **FR-004**: Only `http`, `https`, and `about:blank` pages MUST load in the node; popups and
  new-window links MUST open in the same node; downloads MUST go to the system browser.
- **FR-005**: The node's URL and title MUST be saved with the workspace and reopened on load;
  navigation history is not saved.
- **FR-006**: Browser nodes MUST support move, resize (page reflows), link, select, delete, and a
  floating toolbar with link, reload, open externally, and delete.

**Live pages and cards**

- **FR-007**: A browser page MUST be live only when its node is visible at a legible zoom (the same
  rule as live terminals), no interface element overlaps it, no gesture is in progress, and it is
  among the 4 live browsers allowed (selected first, then largest on screen).
- **FR-008**: When not live, the node MUST show a card with title, URL, and the most recent image of
  the page, captured when the page stops being live and at most every 5 seconds while it is live
  and changing.
- **FR-009**: Hiding a page MUST NOT unload it; deleting the node MUST free it.

**Sessions**

- **FR-010**: Browser nodes of the same workspace MUST share cookies and site storage; different
  workspaces MUST NOT. Deleting a workspace MUST delete its browsing data.
- **FR-011**: Pages MUST NOT be able to call Sonata's internal commands or read Sonata's own
  interface or files.

**Agent control**

- **FR-012**: `sonata browser` commands (open, read, click, fill, press, screenshot, back, forward,
  reload) MUST work only for terminals linked to the browser node; names resolve among linked
  browser nodes by title or by the short id shown in `sonata list`.
- **FR-013**: `sonata list` MUST include linked browser nodes with title and URL.
- **FR-014**: `read` MUST return title, URL, readable text (at most 100 KB, cut with a note), and a
  numbered list of visible interactive elements with their labels; numbers stay valid until the
  next `read` or navigation.
- **FR-015**: `click` and `fill` MUST accept an element number from the last `read` or a CSS
  selector; `fill` MUST produce the same input and change events as typing.
- **FR-016**: Navigation commands MUST wait for the page load (default 30 s, `--timeout` up to
  5 minutes); `click` waits for a navigation or 1 second of network quiet, at most 10 s.
- **FR-017**: `screenshot` MUST save a PNG of the visible page, readable only by the user, in a
  temporary folder or at `--out`, and print the path; it MUST work when the page is hidden.
- **FR-018**: `eval` MUST be allowed only for Orchestrator terminals and return the JSON result
  (at most 1 MB).
- **FR-019**: While an agent controls a browser, its node MUST show "controlled by <terminal>"
  and the link between them MUST be highlighted, as with asks.

### Key Entities

- **Browser node**: a canvas node with URL, title (persisted), and in-memory history; belongs to
  one workspace.
- **Browsing session**: cookies and site storage shared by the browser nodes of one workspace.
- **Element map**: the numbered interactive elements from the last `read` of a browser, in memory.
- **Page image**: the last captured image of a page, used by the card (in memory).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user places a browser and loads a local dev server in under 10 seconds.
- **SC-002**: While panning and zooming a canvas with 6 browser nodes, no page is ever drawn
  outside its node or over a dialog (checked frame by frame in a recording).
- **SC-003**: An agent completes a login-form test (open, read, fill two fields, click, read
  result, screenshot) using only `sonata browser` commands, on the first try, for a local test
  page.
- **SC-004**: Logging in to a site in one browser node keeps the user logged in for another
  browser node of the same workspace, and not in a browser of another workspace.
- **SC-005**: Every `sonata browser` command from an unlinked terminal is refused (tested for each
  command).

## Assumptions

- The live-browser limit is 4 and the legible zoom rule is the one used for terminals.
- Screenshots go to a private temporary folder of the app unless `--out` is given; old ones are
  removed when the app quits.
- Developer tools are available in development builds and enabled in release builds for browser
  nodes only.
- Out of scope: tabs inside a node, a download manager, extensions, device simulators, recording,
  and multi-select.
