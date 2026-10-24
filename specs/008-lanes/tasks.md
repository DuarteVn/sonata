# Tasks: Lanes, an Isolated Git Checkout per Agent Terminal

**Input**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/)

**Tests**: Required (constitution III). Each test task comes before its implementation task and
must fail first. Every git test creates its own temporary repository with a shared helper and
never touches a real repository.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: User story from spec.md (US1–US5)

## Phase 1: Setup

- [x] T001 Add a temporary-repository helper (`git init -b main`, local `user.name`/`user.email`, `commit.gpgsign=false`, helpers `commit_file`, `git(args)`) to crates/core/tests/common/mod.rs
- [x] T002 Create the empty module tree `crates/core/src/lanes/{mod,git,naming,guard,ops,status,confirm}.rs` and register `pub mod lanes;` in crates/core/src/lib.rs

## Phase 2: Foundational (blocks every story)

- [x] T003 [P] Write unit tests for slug and branch naming (ASCII folding, dashes, 40-char cap, empty → `lane`, `-2` suffix when branch or folder is taken) in crates/core/src/lanes/naming.rs (`#[cfg(test)]`)
- [x] T004 [P] Implement naming per research R3 in crates/core/src/lanes/naming.rs
- [x] T005 [P] Write tests for the git runner: env scrubbing (`GIT_DIR` set in the parent is ignored), timeout, version parse (`2.54.0 (Apple Git-157)` → 2.54, < 2.38 rejected), porcelain v2 `-z` status parse, `worktree list --porcelain -z` parse in crates/core/tests/lanes_git.rs
- [x] T006 Implement the runner, git discovery (research R2: login-shell PATH, `/usr/bin/git` only when `xcode-select -p` succeeds), and parsers in crates/core/src/lanes/git.rs
- [x] T007 [P] Write guard tests (research R7): `..`, relative path, sibling prefix `lanes/<id>-x`, nested path, wrong slug, symlink to a real folder, path not in `worktree list`, valid path passes in crates/core/tests/lanes_guard.rs
- [x] T008 Implement `check_removable` in crates/core/src/lanes/guard.rs and `DataRoot::lanes_dir()` / `lane_workspace_dir(id)` in crates/core/src/paths.rs
- [x] T009 [P] Write a test that `enforce_private_tree` fixes `lanes/` and `lanes/<id>` to 0700 but leaves a `0755` script inside `lanes/<id>/<slug>/` unchanged, in crates/core/tests/lanes_guard.rs
- [x] T010 Make `enforce_private_tree` skip lane checkouts (research R4) in crates/core/src/paths.rs
- [x] T011 [P] Write persistence tests: a terminal with `lane` round-trips with sorted keys and omitted empty fields; a v2 file migrates to v3 unchanged; a v3 file read by a build whose current is 2 is a `NewerVersion` failure, in crates/core/tests/lanes_persistence.rs
- [x] T012 Add `Lane` and `TerminalContent.lane` (data-model.md) in crates/core/src/model/node.rs, bump `WORKSPACE_SCHEMA_VERSION` to 3 in crates/core/src/model/workspace.rs, and register the no-op 2→3 step in crates/core/src/persistence/store.rs
- [x] T013 Add error codes `LaneUnavailable`, `LaneBlocked`, `LaneConflict`, `LaneUnsafePath`, `LaneHasChanges`, `ConfirmationRequired`, `LaneConfirmationNeeded` in crates/core/src/error.rs and the matching union in ui/src/ipc/types.ts

**Checkpoint**: naming, git runner, guard, model, and permission-pass fix are green.

## Phase 3: User Story 1 — Start a terminal in its own lane (P1) 🎯 MVP

**Goal**: A new terminal can run in its own worktree and branch.
**Independent Test**: Two lane terminals change the same file; the workspace folder and each lane
see only their own change (quickstart A).

- [x] T014 [P] [US1] Write tests for `lanes::ops::availability(folder)`: not a repo, detached HEAD, merge in progress, git unavailable, ok with base branch, in crates/core/tests/lanes_create.rs
- [x] T015 [P] [US1] Write tests for `create_terminal(.., lane: true)`: branch `lane/<slug>` exists at the base commit, checkout under `lanes/<ws>/<slug>`, `subdirectory` recorded for a workspace in a subfolder, spawn cwd is `checkoutPath/subdirectory`, two lanes isolate the same file, name collision gets `-2`, failure leaves no branch, in crates/core/tests/lanes_create.rs
- [x] T016 [US1] Implement `availability` and `create` (research R5) in crates/core/src/lanes/ops.rs
- [x] T017 [US1] Add `lane: bool` to `NewTerminal`, call `lanes::ops::create`, and use the lane folder in `spawn_spec` in crates/core/src/app.rs
- [x] T018 [P] [US1] Write a test that a restarted app (new `App` on the same data root) starts a lane terminal in its checkout, in crates/core/tests/lanes_create.rs
- [x] T019 [P] [US1] Write tests that `instructions()` and `typed_instructions()` include the lane branch, base, and `sonata lane` commands only when a lane is set, in crates/core/src/messaging/instructions.rs (`#[cfg(test)]`)
- [x] T020 [US1] Add the lane to `SessionInfo` and the text of research R12 in crates/core/src/messaging/instructions.rs; pass it from `prepare_agent_session` and `maybe_type_instructions` in crates/core/src/app.rs
- [x] T021 [P] [US1] Write tests for `recruit --lane` (creates a lane terminal linked to the caller; `--lane` with `--folder` → exit 2; not a repo → exit 1) in crates/core/tests/lanes_cli.rs
- [x] T022 [US1] Parse `--lane` in `recruit` in crates/core/src/messaging/commands.rs and pass it through `App::recruit` in crates/core/src/app.rs; update recruit help in crates/cli/src/main.rs
- [x] T023 [US1] Add `lane_check` and the `lane` argument of `node_create_terminal` in src-tauri/src/commands/lane.rs and src-tauri/src/commands/node.rs; register in src-tauri/src/commands/mod.rs, src-tauri/src/lib.rs, src-tauri/build.rs, src-tauri/capabilities/default.json
- [x] T024 [P] [US1] Add `Lane` types and `laneCheck`, `nodeCreateTerminal({lane})` in ui/src/ipc/types.ts and ui/src/ipc/commands.ts
- [x] T025 [P] [US1] Write tests: the New terminal dialog shows "Run in a lane" enabled with the base branch, disabled with the reason when `laneCheck` says unavailable, locks the folder field when on, and is absent in Edit mode, in ui/tests/terminalDialog.test.tsx
- [x] T026 [US1] Add the checkbox to ui/src/dialogs/TerminalDialog.tsx

**Checkpoint**: US1 works alone (quickstart A steps 1–3).

## Phase 4: User Story 2 — Land (P1)

**Goal**: Confirmed land merges the lane into the base branch; conflicts change nothing.
**Independent Test**: quickstart B.

- [x] T027 [P] [US2] Write tests for `preview(.., Land)`: commit count, changed files of the lane (untracked included), blockers for dirty lane, dirty main (tracked only; untracked main file is not a blocker), wrong branch checked out, merge in progress, missing branch, in crates/core/tests/lanes_land.rs
- [x] T028 [P] [US2] Write tests for `land`: merge commit with message `Merge lane <branch> into <base>` and two parents; zero commits → nothing to land; conflict → `LaneConflict` listing files, and a hash of every file in both checkouts plus `HEAD` is identical before and after; untracked file in main that the merge would overwrite → error and `HEAD` unchanged; two concurrent lands on one repo run one after the other, in crates/core/tests/lanes_land.rs
- [x] T029 [US2] Implement `preview` and `land` with the per-repository lock and `merge-tree` dry run (research R6, R10) in crates/core/src/lanes/ops.rs
- [x] T030 [P] [US2] Write tests for `App::lane_land`: `confirmed: false` → `ConfirmationRequired` and no change; success stops the terminal, removes the checkout through the guard, keeps the branch, sets `landedAt`; restarting a landed terminal clears `lane` and starts in the workspace folder, in crates/core/tests/lanes_land.rs
- [x] T031 [US2] Implement `lane_preview`, `lane_land`, and the landed restart rule in crates/core/src/app.rs
- [x] T032 [US2] Add `lane_preview` and `lane_land` commands in src-tauri/src/commands/lane.rs and their registrations; add wrappers in ui/src/ipc/commands.ts
- [x] T033 [P] [US2] Write tests for `LaneDialog` in land mode: shows branch, base, commit count; disables Land and lists blockers; shows the conflict file list on `LaneConflict`; Cancel calls nothing, in ui/tests/laneDialog.test.tsx
- [x] T034 [US2] Implement ui/src/dialogs/LaneDialog.tsx (land mode) and the "Land…" item in ui/src/nodes/TerminalMenu.tsx; show the landed mark in ui/src/nodes/TerminalNode.tsx

**Checkpoint**: US1 + US2 = MVP.

## Phase 5: User Story 3 — Remove a lane (P2)

**Goal**: Confirmed removal deletes only a verified checkout; the branch stays unless chosen.
**Independent Test**: quickstart C.

- [x] T035 [P] [US3] Write tests for `remove`: clean lane → folder gone, `worktree list` without it, branch kept, terminal back in the workspace folder; dirty lane without `discardChanges` → `LaneHasChanges` and nothing removed; with it → removed; `deleteBranch` on a merged branch uses `-d`; on an unmerged branch deletes only with the flag and reports the lost count; hand-edited `checkoutPath` pointing at the main checkout or outside `lanes/` → `LaneUnsafePath`, nothing deleted; missing folder → prune and clear (forget), in crates/core/tests/lanes_remove.rs
- [x] T036 [US3] Implement `remove` and `forget` (research R7, R8) in crates/core/src/lanes/ops.rs and `App::lane_remove`, `App::lane_forget` in crates/core/src/app.rs
- [x] T037 [P] [US3] Write tests: `delete_node` on a lane terminal without `lane_removal` → `LaneConfirmationNeeded`; with it → lane removed then node deleted; `delete_workspace` with a dirty lane → `LaneHasChanges` and nothing deleted; with clean lanes → lanes removed and `lanes/<ws>` gone, in crates/core/tests/lanes_remove.rs
- [x] T038 [US3] Apply research R14 in `delete_node` and `delete_workspace` in crates/core/src/app.rs; pass `laneRemoval` through src-tauri/src/commands/node.rs
- [x] T039 [US3] Add `lane_remove`, `lane_forget` commands and wrappers in src-tauri/src/commands/lane.rs, ui/src/ipc/commands.ts
- [x] T040 [P] [US3] Write tests for `LaneDialog` remove mode (changed-file list, second "discard these changes" confirmation, delete-branch option with unmerged-commit warning) and for node deletion opening it, in ui/tests/laneDialog.test.tsx
- [x] T041 [US3] Implement remove mode in ui/src/dialogs/LaneDialog.tsx, "Remove lane…" and "Forget lane" in ui/src/nodes/TerminalMenu.tsx, and the lane path in node and workspace deletion in ui/src/state/canvas.ts, ui/src/canvas/Canvas.tsx, ui/src/canvas/NodeToolbar.tsx, and ui/src/sidebar/Sidebar.tsx

## Phase 6: User Story 4 — Lane state on the canvas (P2)

**Goal**: Node shows branch, changed count, ahead/behind, broken states within 5 s.
**Independent Test**: quickstart A step 3 and a commit observed on the badge.

- [x] T042 [P] [US4] Write tests for `status::check(lane)`: counts for changed/ahead/behind; `checkoutMissing`, `branchMissing`, `mainCheckoutMissing`, `busy`; and for the poller: emits `lane://status` once per change, not on an unchanged poll, in crates/core/tests/lanes_status.rs
- [x] T043 [US4] Implement `status.rs` (research R11), start the loop in `App::start_with`, and trigger immediate checks after create/land/remove/exit in crates/core/src/app.rs; add `CoreEvent::LaneStatus` in crates/core/src/events.rs
- [x] T044 [P] [US4] Write tests for `LaneBadge` (branch, `●N`, `↑a ↓b`, broken message) in ui/tests/laneBadge.test.tsx
- [x] T045 [US4] Implement ui/src/nodes/LaneBadge.tsx, subscribe to `lane://status` in ui/src/ipc/events.ts and the store in ui/src/state/workspaces.ts, render it in ui/src/nodes/TerminalNode.tsx

## Phase 7: User Story 5 — Agents request land and removal (P3)

**Goal**: `sonata lane status|land|remove` with a human confirmation in the app.
**Independent Test**: quickstart D.

- [x] T046 [P] [US5] Write tests for `Confirmations`: answer accepted/declined, timeout returns declined and emits `lane://confirm-closed`, second request for the same lane refused, in crates/core/src/lanes/confirm.rs (`#[cfg(test)]`)
- [x] T047 [US5] Implement crates/core/src/lanes/confirm.rs and `CoreEvent::LaneConfirm`, `LaneConfirmClosed` in crates/core/src/events.rs
- [x] T048 [P] [US5] Write CLI tests with a fake answering sink: `lane status` own and linked; `lane land` by a non-Orchestrator for another terminal → exit 3; declined → exit 3 and no change; accepted → landed and the reply is returned before the caller stops; `lane remove --delete-branch`; `dismiss` of a lane terminal asks first, in crates/core/tests/lanes_cli.rs
- [x] T049 [US5] Implement `lane status|land|remove` and the dismiss confirmation in crates/core/src/messaging/commands.rs; add `lane` help in crates/cli/src/main.rs
- [x] T050 [US5] Add `lane_confirm_answer` in src-tauri/src/commands/lane.rs with registrations; listen to `lane://confirm` and `lane://confirm-closed` in ui/src/ipc/events.ts and open `LaneDialog` from ui/src/App.tsx
- [x] T051 [P] [US5] Write a test that a `lane://confirm` event opens the dialog with "requested by <name>" and answering calls `laneConfirmAnswer`, in ui/tests/laneDialog.test.tsx

## Phase 8: Polish

- [x] T052 [P] Add a Lanes section to README.md (how to create, land, remove; where checkouts live; that only git-tracked work moves; manual cleanup of old hand-made worktrees)
- [x] T053 Run quickstart scenarios A–E with `SONATA_HOME="$(mktemp -d)/sonata" npm run tauri dev` and note results in specs/008-lanes/quickstart.md
- [x] T054 Run `sh scripts/check.sh` and fix every failure

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories. US1 first (every other story needs a lane to exist).
- US2 and US3 share `ops.rs` and `LaneDialog.tsx`: do US2 then US3.
- US4 needs only US1. US5 needs US2 and US3 (it calls land and remove).
- Within a story: tests → core → shell commands → UI.

## Parallel Examples

- Phase 2: T003, T005, T007, T009, T011 (separate test files) together; then T004, T006, T008.
- US1: T014, T015, T018, T019, T021 together; T024 and T025 alongside core work.
- US4 can run beside US3 once US1 is done.

## Implementation Strategy

1. MVP = Phases 1–4 (create + land). Stop and validate quickstart A and B.
2. Add US3 (remove), then US4 (badge), then US5 (CLI).
3. Commit after each task or small group; Conventional Commits.
