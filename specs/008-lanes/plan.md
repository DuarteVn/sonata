# Implementation Plan: Lanes, an Isolated Git Checkout per Agent Terminal

**Branch**: `008-lanes` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)

## Summary

A terminal can be created in a lane: `core` runs `git worktree add -b lane/<slug>` into
`<data root>/lanes/<workspace id>/<slug>` and starts the terminal there. Land checks both
checkouts, dry-runs the merge with `git merge-tree` so a conflict changes no file, then makes a
`--no-ff` merge commit in the workspace folder and removes the lane. Removal lets git delete the
folder only after a four-step path guard. Every land and removal is confirmed by a human; requests
from the `sonata` command wait for a dialog in the app. A 3-second poll shows branch, changed
files, and ahead/behind on the node. See [research.md](research.md).

## Technical Context

**Language/Version**: Rust 2024 edition (rustc 1.99), TypeScript 7 / React 19.
**Primary Dependencies**: none new. System `git` ≥ 2.38 through `std::process::Command`.
**Storage**: `TerminalContent.lane` in the workspace file (schema 2 → 3); checkouts in
`<data root>/lanes/`. **Testing**: `cargo test` integration tests that create temporary git
repositories (`git init` in a `tempfile` dir, with `user.name`/`user.email` set locally); Vitest +
Testing Library for the dialog, badge, and menu. **Target Platform**: macOS 15+.
**Project Type**: desktop app (Tauri) + CLI. **Performance Goals**: status within 5 s
(SC-007); lane creation under 10 s for 10,000 files (SC-001). **Constraints**: no file of either
checkout changes on a failed land (SC-003); nothing deleted outside the lane location (SC-004).
**Scale/Scope**: a handful of lanes per workspace.

## Constitution Check

| Principle | Result |
|---|---|
| I Clean room | Pass: own design (lane per terminal, not separate canvases), own names and text. Git worktrees are a generic technique. |
| II Core/shell | Pass: git, guard, land, confirmations, and permissions in `core::lanes`; `src-tauri` only forwards commands and events. |
| III Test-first | Pass: every git behavior is tested in a temporary repository created by the test; guard and naming are pure unit tests; UI tests before components. |
| IV Files | Pass: optional field, schema bump with migration step, envelope unchanged; broken lanes are reported, never auto-deleted. |
| V Security | Pass: path guard in code (research R7), git deletes only its own registered worktree, human confirmation in UI and CLI, permissions decided by the server from the caller's identity and links. |
| VI Distribution | Pass: no new service or dependency. |
| VII Simplicity | Pass: no new crate; worktrees only (no copy mode, hooks, PRs). |

Re-checked after design: no change.

## Project Structure

### Documentation (this feature)

```text
specs/008-lanes/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/cli.md  contracts/ipc.md
└── tasks.md            # /speckit-tasks
```

### Source Code

```text
crates/core/src/lanes/
├── mod.rs        # Lane model helpers, LaneStatus, public API used by App
├── git.rs        # runner (env, timeout), git discovery (R1, R2), porcelain parsers
├── naming.rs     # slug and unique branch/folder names (R3)
├── guard.rs      # check_removable (R7)
├── ops.rs        # create, preview, land, remove, forget (R5, R6, R8, R10)
├── status.rs     # poll loop and cache (R11)
└── confirm.rs    # Confirmations registry (R9)
crates/core/src/model/node.rs         # TerminalContent.lane, Lane
crates/core/src/model/workspace.rs    # schema 3
crates/core/src/persistence/store.rs  # v2 → v3 step
crates/core/src/paths.rs              # lanes_dir(); enforce_private_tree skips checkouts (R4)
crates/core/src/app.rs                # create_terminal(lane), spawn cwd, delete_node, delete_workspace, recruit, dismiss
crates/core/src/events.rs             # LaneStatus, LaneConfirm, LaneConfirmClosed
crates/core/src/error.rs              # new error codes
crates/core/src/messaging/commands.rs # `lane …`, `recruit --lane`, dismiss confirmation
crates/core/src/messaging/instructions.rs # lane section (R12)
crates/cli/src/main.rs                # help text
crates/core/tests/lanes_*.rs          # git, guard, land, remove, cli, persistence
src-tauri/src/commands/lane.rs        # lane_* commands
src-tauri/build.rs, capabilities/default.json, src/lib.rs
ui/src/ipc/{commands,types,events}.ts
ui/src/dialogs/TerminalDialog.tsx     # "Run in a lane" checkbox
ui/src/dialogs/LaneDialog.tsx         # land / remove / CLI-request confirmation
ui/src/nodes/LaneBadge.tsx            # branch, changed, ahead/behind, state
ui/src/nodes/TerminalMenu.tsx         # Land…, Remove lane…, Forget lane
ui/tests/lane*.test.tsx
README.md                             # Lanes section
```

**Structure Decision**: Existing layout; one new `core` module folder and one new shell command
file.

## Phases

1. **Foundation**: git runner and discovery, naming, guard, model + schema 3, permission-pass
   exclusion.
2. **US1 create** (P1): `create_terminal` with lane, spawn cwd, `lane_check`, dialog checkbox,
   `recruit --lane`, agent instructions.
3. **US2 land** (P1): preview, land with dry run and lock, landed state, UI dialog.
4. **US3 remove** (P2): remove/forget, delete node and workspace rules.
5. **US4 status** (P2): poll loop, events, badge.
6. **US5 CLI** (P3): confirmations registry, `sonata lane …`, dismiss confirmation.
7. **Polish**: README, quickstart run, `scripts/check.sh`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Background status thread | Node status within 5 s without user action (FR-018). | File watchers on whole checkouts are heavier; refresh-on-click misses agent commits. |
| In-memory confirmation registry | CLI requests must wait for a human answer in the app (FR-022). | Letting the CLI prompt in the terminal would let the agent answer its own prompt. |
