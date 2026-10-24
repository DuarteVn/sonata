# Feature Specification: Lanes, an Isolated Git Checkout per Agent Terminal

**Feature Branch**: `008-lanes`

**Created**: 2026-10-23

**Status**: Draft

**Input**: User description: "Each agent can work in a lane: its own git worktree of the
workspace's project, on its own branch. Several agents change the same repository in parallel
without stepping on each other. When the work is done, land merges it back into the base branch.
Then the lane is removed."

Builds on 001–003 (workspaces with a working directory, terminal nodes, the `sonata` command with
link-based permissions and the Orchestrator flag).

## Clarifications

### Session 2026-10-23

- Q: Where are lane checkouts stored? → A: Under the Sonata data folder,
  `~/.sonata/lanes/<workspace>/<lane>`.
- Q: How does land bring the work into the base branch? → A: A merge commit; lane commits are kept.
- Q: May agents request land and removal from the command line? → A: Yes. The app shows a
  confirmation dialog to the human and the command waits for the answer.
- Q: What happens to the terminal after a successful land? → A: The lane is removed and the
  terminal's process is stopped; the node stays on the canvas marked as landed, and the user
  restarts it in the workspace folder or deletes it.
- Q: Can an existing terminal be moved into a lane? → A: No. A lane is chosen only when a terminal
  is created (New terminal dialog or `sonata recruit`).
- Q: Is the agent told it works in a lane? → A: Yes. The instructions Sonata already gives agents
  add the lane branch, the base branch, a reminder to commit work, and how to see lane status and
  request land.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Start a terminal in its own lane (Priority: P1)

A developer has a workspace whose folder is a git repository. They want two agents to work on two
features at the same time. In the New terminal dialog they turn on "Run in a lane". Sonata creates
a separate checkout of the repository on a new branch, and the terminal starts inside it. The
second agent gets its own lane the same way. Each agent edits, builds, and commits without seeing
the other's uncommitted files, and the developer's own checkout stays untouched.

An Orchestrator terminal can do the same from the command line when it recruits a helper.

**Why this priority**: Isolation is the reason the feature exists. Without it, parallel agents
overwrite each other's files, which is what the user does by hand today with sibling folders.

**Independent Test**: In a temporary repository, create two terminals with lanes. In each, change
the same file differently and commit. Check that the workspace folder, the first lane, and the
second lane each show only their own change, and that two new branches exist.

**Acceptance Scenarios**:

1. **Given** a workspace whose folder is inside a git repository with a branch checked out,
   **When** the user creates a terminal with "Run in a lane" on, **Then** a new branch and a new
   checkout are created from the current commit of that branch, and the terminal starts in the
   checkout.
2. **Given** the workspace folder is a subfolder of the repository, **When** a lane is created,
   **Then** the terminal starts in the same subfolder inside the lane checkout.
3. **Given** a workspace folder that is not inside a git repository, **When** the user opens the
   New terminal dialog, **Then** the lane option is disabled and says why.
4. **Given** an Orchestrator terminal, **When** it recruits a new agent with the lane option,
   **Then** the new terminal is created in its own lane and linked to the caller, as with any
   recruit.
5. **Given** a terminal in a lane, **When** the app restarts, **Then** the terminal comes back in
   the same lane checkout.

---

### User Story 2 - Land the lane's work into the base branch (Priority: P1)

An agent finishes its feature and commits in its lane. The developer chooses "Land" on the
terminal. Sonata shows what will happen (lane branch, base branch, number of commits) and asks for
confirmation. On confirmation, the lane's commits are brought into the base branch in the
workspace folder. If that cannot be done cleanly, nothing changes and the developer sees why.

**Why this priority**: Without land, the user must merge by hand, and the feature saves little
work. Land and creation together make the minimum useful slice.

**Independent Test**: In a temporary repository, create a lane, commit a change in it, land it,
and check that the base branch now contains the change. Repeat with a conflicting change made on
the base branch first, and check that the base branch and the workspace folder are unchanged after
the failed land.

**Acceptance Scenarios**:

1. **Given** a lane with commits that the base branch does not have, and clean checkouts on both
   sides, **When** the user confirms Land, **Then** the base branch contains the lane's work using
   a merge commit (the lane's commits are kept as they are), the lane is then removed as in
   User Story 3, and the terminal node stays on the canvas marked as landed.
2. **Given** the land would produce conflicts, **When** the user confirms Land, **Then** the
   operation is cancelled, the base branch and the workspace folder are exactly as before, and the
   user sees the list of conflicting files.
3. **Given** the lane or the workspace folder has uncommitted changes, **When** the user chooses
   Land, **Then** Land is refused before any change, and the message says which side is not clean.
4. **Given** the workspace folder has a different branch checked out than the lane's base branch,
   **When** the user chooses Land, **Then** Land is refused and the message names both branches.
5. **Given** the lane has no commits that the base branch lacks, **When** the user chooses Land,
   **Then** Sonata says there is nothing to land and offers to remove the lane.
6. **Given** the user cancels the confirmation, **Then** nothing changes.

---

### User Story 3 - Remove a lane (Priority: P2)

Work in a lane was abandoned, or it was landed. The developer removes the lane. Sonata deletes the
lane checkout from disk and keeps the branch, so no commit is lost. The terminal node stays on the
canvas and works in the workspace folder again.

**Why this priority**: Lanes take disk space and clutter the branch list. Removal is needed for
long-term use, but a user can live a few days without it.

**Independent Test**: In a temporary repository, create a lane, remove it, and check that the
checkout folder is gone, git no longer lists it, the branch still exists, and no other folder was
touched.

**Acceptance Scenarios**:

1. **Given** a lane with a clean checkout, **When** the user confirms Remove lane, **Then** the
   terminal's process is stopped, the checkout folder is deleted, git forgets it, the branch is
   kept, and the terminal returns to the workspace folder.
2. **Given** a lane checkout with uncommitted changes, **When** the user chooses Remove lane,
   **Then** Sonata lists the changed files and removes the lane only after a second, explicit
   confirmation that these changes will be lost.
3. **Given** the confirmation offers "Also delete the branch", **When** the user selects it and the
   branch has commits that no other branch contains, **Then** Sonata warns that those commits will
   be lost before it deletes the branch.
4. **Given** a lane record points to a folder that is not a checkout Sonata created for this
   repository, **When** removal runs, **Then** Sonata refuses to delete it and reports why.
5. **Given** a terminal with a lane, **When** the user deletes the terminal node, **Then** the lane
   is removed under the same rules and confirmations as Remove lane.

---

### User Story 4 - See each lane's state on the canvas (Priority: P2)

Looking at the canvas, the developer sees which terminals work in a lane, on which branch, whether
the lane has uncommitted changes, and how many commits it is ahead of and behind the base branch.
This tells them which agent is ready to land.

**Why this priority**: Useful for overview, but creation and land work without it.

**Independent Test**: Create a lane, make an uncommitted change, then commit it. Check that the
node shows the change state and then "1 ahead" within a few seconds, without user action.

**Acceptance Scenarios**:

1. **Given** a terminal in a lane, **Then** its node shows the lane branch name.
2. **Given** the lane checkout has uncommitted changes, **Then** the node shows a changed-files
   indicator with the count.
3. **Given** the lane has commits the base lacks, or the base has commits the lane lacks, **Then**
   the node shows the ahead and behind counts.
4. **Given** the state changes in the checkout, **Then** the node updates within 5 seconds.

---

### User Story 5 - Agents ask for land and removal from the command line (Priority: P3)

An agent working in a lane finishes its task and asks Sonata to land its work, or an Orchestrator
asks to land a helper's lane. The app shows a confirmation dialog to the human, and the command
waits for the answer. A human still confirms every land and removal.

**Why this priority**: It saves clicks in long agent sessions, but the UI flow covers the need.

**Independent Test**: From a terminal in a lane, run the land request. Check that the app shows the
confirmation, that declining leaves everything unchanged, and that the command reports the answer.

**Acceptance Scenarios**:

1. **Given** any terminal in a lane, **When** it asks for its own lane status from the command
   line, **Then** it gets the branch, base branch, changed-file count, and ahead and behind counts.
2. **Given** a land or remove request from the command line, **When** the human declines or does
   not answer before the timeout, **Then** nothing changes and the command reports it.

---

### Edge Cases

- The workspace folder has a detached HEAD (no branch checked out): lane creation is refused with a
  message, because there is no base branch to land into.
- The repository is in the middle of a merge, rebase, or cherry-pick: lane creation and land are
  refused until it ends.
- The generated branch name or checkout folder already exists: Sonata picks the next free name and
  never reuses or overwrites an existing branch or folder.
- The lane checkout folder was deleted or moved outside Sonata: the node shows "lane checkout
  missing", the terminal does not start, and the user can forget the lane (the branch is kept).
- The workspace folder moves or is no longer a repository: lane nodes show the error; nothing is
  deleted automatically.
- Two lands into the same repository at the same moment: they run one after the other.
- A lane's branch was deleted outside Sonata: the node shows the error; land is refused.
- The repository uses submodules: the lane checkout includes the submodule pointers, but submodule
  contents are not set up (documented limitation).
- Ignored files (dependencies, build output, local secrets files) are not present in a new lane;
  the agent installs or creates them as in a fresh clone.
- The user deletes a workspace that has lanes: the confirmation lists the lanes; clean lanes are
  removed with the workspace; a lane with uncommitted changes blocks the deletion until it is
  handled.
- A lane path stored in the workspace file is edited by hand to point elsewhere: removal refuses
  any folder outside the managed lane location or not registered by git as a checkout of this
  repository.

## Requirements *(mandatory)*

### Functional Requirements

**Creation**

- **FR-001**: The New terminal dialog MUST offer a "Run in a lane" option, enabled only when the
  workspace folder is inside a git repository with a branch checked out.
- **FR-002**: The `sonata recruit` command MUST accept a lane option with the same effect, subject
  to the existing Orchestrator permission.
- **FR-003**: Creating a lane MUST create a new branch from the current commit of the branch
  checked out in the workspace folder (the base branch) and a new checkout of that branch, and
  MUST record the base branch for the lane.
- **FR-004**: The lane branch name MUST be derived from the terminal name with a fixed `lane/`
  prefix, made safe for git, and made unique; existing branches and folders MUST NOT be reused.
- **FR-005**: Lane checkouts MUST be created only inside one Sonata-managed lane location: the
  `lanes` folder in the Sonata data folder (`~/.sonata/lanes/<workspace>/<lane>`). The user's
  repository folder and its parent get no extra folders.
- **FR-006**: A terminal in a lane MUST start in the lane checkout, in the same relative subfolder
  as the workspace folder has in the repository.
- **FR-007**: Each lane MUST belong to exactly one terminal node; one terminal has at most one lane.
  A lane is chosen only when the terminal is created; the Edit terminal dialog does not add or
  change a lane.

**Land**

- **FR-008**: Land MUST show the lane branch, base branch, and commit count, and MUST run only
  after explicit human confirmation.
- **FR-009**: Land MUST be refused, before any change, when the lane checkout or the workspace
  folder has uncommitted changes, when the workspace folder is not on the base branch, when the
  repository is mid-operation, or when the lane branch is missing.
- **FR-010**: When land cannot complete without conflicts, Sonata MUST restore the base branch and
  the workspace folder to their exact prior state and list the conflicting files (at least the
  first 10 and the total count).
- **FR-011**: After a successful land, Sonata MUST remove the lane as in FR-013 and keep the
  branch. The terminal node MUST stay on the canvas, stopped and marked as landed, until the user
  restarts it (it then runs in the workspace folder) or deletes it. A land requested by the lane's
  own terminal MUST report the result to that command before the process is stopped.
- **FR-012**: Lands into the same repository MUST run one at a time.

**Removal**

- **FR-013**: Remove lane MUST require explicit human confirmation, stop the terminal's process,
  delete the checkout folder, make git forget the checkout, keep the branch by default, and return
  the terminal to the workspace folder.
- **FR-014**: When the lane checkout has uncommitted changes, removal MUST list them and require a
  second explicit confirmation.
- **FR-015**: Deleting the branch MUST be an opt-in choice in the removal confirmation, with a
  warning when the branch has commits no other branch contains.
- **FR-016**: Before deleting any folder, Sonata MUST check in code that the path has no `..`
  component, lies strictly inside the managed lane location, is recorded as this lane's checkout,
  and is registered by git as a checkout of the workspace's repository. If any check fails,
  nothing is deleted and the user sees why.
- **FR-017**: Deleting a terminal node that has a lane MUST remove the lane under FR-013 to FR-016.

**Status and persistence**

- **FR-018**: A terminal node in a lane MUST show the lane branch, the uncommitted-change count, and
  the ahead and behind counts relative to the base branch, updated within 5 seconds of a change.
- **FR-019**: Lane data (branch, base branch, checkout path) MUST be stored in the workspace file
  with the terminal, in the existing envelope with a schema migration, and MUST survive restarts.
- **FR-020**: A lane whose checkout or branch is missing MUST be shown as broken, MUST NOT crash
  the app or be deleted automatically, and MUST offer "Forget lane", which keeps the branch.
- **FR-021**: Any terminal in a lane MUST be able to read its own lane status through the `sonata`
  command.
- **FR-021a**: When a terminal in a lane starts an agent, the instructions Sonata gives the agent
  MUST state the lane branch and base branch, ask the agent to commit its work in the lane and not
  switch branches, and explain how to read lane status and request land.
- **FR-022**: A terminal MAY request land or removal of its own lane through the `sonata` command;
  an Orchestrator terminal MAY request it for the lane of a terminal linked to it. The app MUST
  show the same confirmation as the UI, the command MUST wait for the answer, and a decline or a
  timeout MUST leave everything unchanged.

**Platform rules**

- **FR-023**: All git operations MUST use the system `git` program; when git is missing, lane
  options are disabled with an explanation.
- **FR-024**: Lane errors MUST show a short message in the app and in the CLI output, with the git
  error text available for detail.

### Key Entities

- **Lane**: An isolated checkout of the workspace's repository owned by one terminal. Attributes:
  branch name, base branch, checkout path, creation time, state (ok, checkout missing, branch
  missing).
- **Base branch**: The branch checked out in the workspace folder when the lane was created; land
  targets it.
- **Lane status**: Derived, not stored. Uncommitted-change count, ahead count, behind count,
  conflict or mid-operation flag.
- **Managed lane location**: The single folder under which Sonata creates lane checkouts; the
  safety boundary for removal.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user creates a terminal in a lane with one extra click (one checkbox) compared to a
  normal terminal, and the terminal is ready in under 10 seconds for a repository of up to 10,000
  tracked files.
- **SC-002**: With two lanes changing the same file, no change from one lane appears in the other
  lane or in the workspace folder before land, in 100% of test runs.
- **SC-003**: After a failed land (conflict or refusal), the base branch and the workspace folder
  are byte-identical to their state before the attempt, in 100% of test runs.
- **SC-004**: In tests with crafted paths (`..`, symlinks, prefix look-alikes, hand-edited records),
  removal deletes nothing outside the managed lane location: 0 out-of-bounds deletions.
- **SC-005**: No land or removal happens without a human confirmation: 0 cases in tests of the UI
  and CLI paths.
- **SC-006**: A developer lands a finished lane in under 30 seconds from opening the action to
  seeing the work on the base branch.
- **SC-007**: Node status reflects a commit or file change within 5 seconds.

## Assumptions

- One lane per terminal. Sharing one lane among several terminals is out of scope.
- Lanes start from the base branch's last commit. Uncommitted changes in the workspace folder are
  not carried into the lane.
- No setup or teardown commands run when a lane is created or removed (for example, installing
  dependencies); the agent does that in its terminal. This can be a later feature.
- Only git worktrees are used. Copy-based isolation for non-git folders is out of scope.
- Lanes work only for local workspaces on this Mac.
- Pull requests and remote pushes are out of scope; the user or agent pushes the lane branch by
  hand if wanted.
- Resolving land conflicts is done by the agent or user inside the lane (for example, by bringing
  the base branch into the lane and committing), then landing again. Sonata does not offer a
  conflict editor.
- Status refresh uses periodic checks plus checks after known events (terminal output idle,
  focus); exact cadence is a plan decision within the 5-second target.
- Closing the app or stopping the terminal never removes a lane.
