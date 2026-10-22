# Quickstart: Validate Feature 001

How to build, run, and prove the feature works. Requirement ids refer to [spec.md](spec.md).

## Prerequisites

- macOS 15 or later, Xcode Command Line Tools (`xcode-select --install`).
- Rust stable (`rustup`), Node 22 or later with npm.
- Use a throwaway data folder so tests never touch real data:

```sh
export SONATA_HOME="$(mktemp -d)/sonata"
```

## Build and automated checks

```sh
npm ci
cargo fmt --check && cargo clippy --workspace -- -D warnings
cargo test --workspace            # unit + integration (core), temp SONATA_HOME per test
npm run typecheck && npm test     # vitest (frontend)
npx playwright test               # canvas and gestures against mocked IPC
npm run tauri dev                 # run the app
```

All commands must pass before merge (constitution, Quality Gates).

## Manual scenarios

### 1. First run and terminals (US1)

1. Start with an empty `SONATA_HOME`. **Expect**: empty state with "Create workspace".
2. Create workspace "Demo" with an existing folder. Add a terminal; run `pwd`. **Expect**: the
   folder path (FR-010).
3. Add a terminal with launch command `echo hello`. **Expect**: `hello` printed, shell stays open
   (FR-011).
4. Move, resize, rename, and bring a terminal to front; pan and zoom. Restart the app. **Expect**:
   identical layout and camera (FR-008, FR-009).
5. Resize a terminal running `vim`. **Expect**: vim redraws to the new size.

### 2. Many workspaces (US2)

1. Create 5 workspaces. Try an empty name, a duplicate name in different case, and a missing
   folder. **Expect**: three distinct error messages, nothing created (FR-002).
2. **Expect**: five different colors (FR-003).
3. In workspace A run `for i in $(seq 1 120); do echo $i; sleep 1; done`. Switch to B and C for
   one minute, then back. **Expect**: output continued, nothing missing (FR-005, FR-013).
4. Pin, unpin, reorder; restart. **Expect**: same order, pinned first (FR-004).
5. Delete a workspace, cancel, then delete and confirm. **Expect**: cancel changes nothing;
   confirm removes `$SONATA_HOME/workspaces/<id>` and leaves the project folder intact (FR-006).

### 3. Restart and crash recovery (US3)

1. Change a layout, wait 30 s, `kill -9` the app. Restart. **Expect**: change kept (FR-024).
2. Change a layout and `kill -9` within 5 s. **Expect**: at most that change lost (SC-005).
3. Leave the app idle 2 minutes and watch `stat -f %i` of a `workspace.json`. **Expect**: inode
   unchanged (no rewrite of unchanged workspaces).
4. Restart with terminals that had output. **Expect**: old output above a "restored history"
   separator, a fresh shell, and launch commands re-run (FR-015, FR-016).
5. Corrupt one `workspace.json` (`echo '{' > …`). Restart. **Expect**: other workspaces load; the
   broken one shows as a load failure with path; the file is unchanged (FR-025, SC-006).
6. Quit Sonata, then set `"consecutiveEarlyCrashes": 1`, `"cleanShutdown": false`, and
   `"outlivedLaunchWindow": false` in `app-state.json` (one early crash already counted, plus the
   one this edit simulates). Start.
   **Expect**: no workspace auto-opened, message explains why (FR-027).
7. `ls -l` under `$SONATA_HOME`. **Expect**: folders `drwx------`, files `-rw-------` (FR-015a).

### 4. Notes (US4)

1. Add a note, type `# Title` and a list, click outside. **Expect**: rendered Markdown (FR-019).
2. **Expect**: `notes/<id>.md` with the same text (FR-020).
3. With the note not in edit mode, run `echo "external" >> notes/<id>.md`. **Expect**: the note
   shows the text within 2 s (FR-020a).
4. Start editing the note, append externally, keep editing. **Expect**: in-app text kept,
   `<id> (conflict).md` created, warning shown (FR-020a).

### 5. Gestures (FR-008a)

1. Focus a terminal with long output; two-finger scroll over it. **Expect**: terminal scrolls.
2. Scroll over an unfocused terminal, a note, and empty space. **Expect**: canvas pans.
3. Pinch. **Expect**: zoom around the pointer. Space + drag with nothing focused, and middle-drag
   anywhere. **Expect**: canvas pans; with a terminal focused, Space types a space.

### 6. Scale (US5, SC-003, SC-004)

1. Run `scripts/bench-terminals.sh 30` (opens 30 terminals printing continuously). Pan and zoom.
   **Expect**: smooth movement; typing in the focused terminal echoes at once.
2. Zoom out until all are previews, wait, zoom into one. **Expect**: current, complete output.
3. Run `scripts/bench-terminals.sh 100` across 4 workspaces. **Expect**: every terminal correct
   when brought into view.

### 7. Quit and close (FR-018, FR-018a)

1. Close a terminal at an idle prompt. **Expect**: closes without asking.
2. Close a terminal running `sleep 100`. **Expect**: confirmation.
3. Cmd+Q with `sleep 100` running in two workspaces. **Expect**: dialog listing the count per
   workspace. With only idle shells, Cmd+Q quits without asking.
