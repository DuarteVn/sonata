# Research: Lanes

**Feature**: 008-lanes | **Date**: 2026-10-23 · Checked against the local Apple Git 2.54 and the
current `core` sources (`paths.rs`, `app.rs`, `messaging/`).

## R1. Running git

**Decision**: A small runner in `core::lanes::git` calls the system `git` with
`std::process::Command`, always with `-C <dir>`, a timeout (30 s; 120 s for `worktree add`), and
this environment: `GIT_TERMINAL_PROMPT=0`, `GIT_OPTIONAL_LOCKS=0` (status polling must not take the
index lock), `LC_ALL=C` (stable messages), and with `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`
removed (a terminal that started the app must not redirect it). Output is parsed only from
porcelain or plumbing formats (`status --porcelain=v2 -z`, `worktree list --porcelain -z`,
`rev-list --count`, `merge-tree --name-only -z`).

**Rationale**: Constitution: system git, no libgit2. Plumbing formats are stable across versions.

**Alternatives considered**: `gix`/libgit2 (forbidden by the constitution); parsing human output
(breaks across versions and locales).

## R2. Finding git without a system prompt

**Decision**: Resolve `git` once per app run. If the login-shell `PATH` (the same lookup that
`presets::detect` uses) has a `git` other than `/usr/bin/git`, use it. If only `/usr/bin/git`
exists, use it only when `xcode-select -p` succeeds; otherwise lanes are unavailable with the
message "Git is not installed. Install the Command Line Tools: xcode-select --install".
Minimum version 2.38 (needed for R6); an older git disables lanes with a message.

**Rationale**: On macOS, `/usr/bin/git` is a shim that opens an install dialog when the Command
Line Tools are missing. Probing it blindly would show that dialog on every start (FR-023).

## R3. Lane location and names

**Decision**: Checkouts live at `<data root>/lanes/<workspace id>/<slug>`. `slug` comes from the
terminal name: lowercase ASCII letters, digits and `-`, other characters become `-`, repeats and
edge dashes removed, at most 40 characters, `lane` when empty. The branch is `lane/<slug>`. If
the branch or the folder exists, `-2`, `-3`, … is appended until both are free. The final name is
checked with `git check-ref-format --branch`.

The folder is not pre-created: `git worktree add` creates it and fails if it exists, so a race
cannot reuse a folder. `<data root>/lanes/<workspace id>` is created with mode `0700`; the
checkout inside keeps the repository's normal file modes.

**Rationale**: Clarification Q1. One fixed boundary makes the removal guard simple (R7). A
workspace id in the path keeps workspaces apart and lets workspace deletion find its lanes.

## R4. Excluding lanes from the startup permission pass

**Decision**: `paths::enforce_private_tree` skips `<data root>/lanes/*/*` (the checkouts). It
still fixes `<data root>/lanes` and `<data root>/lanes/<workspace id>` to `0700`.

**Rationale**: Today the pass sets every file under the data root to `0600` on each start. On a
checkout it would remove execute bits from scripts (`scripts/check.sh`, `node_modules/.bin`) and
walk very large trees at startup. Found while reading `app.rs:271`.

## R5. Creating a lane

**Decision**: Preconditions are checked in this order, each with its own message: git available
(R2); workspace folder inside a work tree (`rev-parse --show-toplevel`); a branch is checked out
(`symbolic-ref -q HEAD`); no operation in progress (no `MERGE_HEAD`, `CHERRY_PICK_HEAD`,
`REVERT_HEAD`, `rebase-merge`, `rebase-apply` in `rev-parse --git-dir`). Then:

```text
git -C <workspace folder> rev-parse HEAD                     → base commit
git -C <repo top> worktree add -b lane/<slug> <path> <base commit>
```

The lane records the main checkout (`--show-toplevel` of the workspace folder), the base branch
name, and the workspace folder's path relative to the top (`subdirectory`, FR-006). If the
workspace folder is itself a linked worktree, its top is used as the main checkout; this is
allowed.

If `worktree add` fails after creating the branch, the branch is deleted again only if it still
points at the base commit (nothing lost).

## R6. Land without touching files on conflict

**Decision**: One mutex per repository (keyed by the canonical `--git-common-dir`) serializes land
(FR-012). Under it:

1. Refuse when: lane checkout has any change (`status --porcelain=v2 -z --untracked-files=normal`
   is not empty, so new files that were never added count); main checkout has tracked changes
   (same command with `--untracked-files=no`); main checkout HEAD is not `refs/heads/<base>`; an
   operation is in progress (R5); the lane branch is missing.
2. Count commits: `rev-list --count <base>..<lane>`. Zero → "Nothing to land" (US2 #5).
3. Dry run: `git merge-tree --write-tree --name-only -z <base> <lane>`. Exit 1 means conflicts;
   the conflicting paths are listed and nothing in either checkout changed (FR-010, SC-003).
4. Record `rev-parse HEAD` of the main checkout, then
   `git -C <main> merge --no-ff --no-edit -m "Merge lane <branch> into <base>" <branch>`.
5. If step 4 fails anyway (an untracked file in the main checkout would be overwritten, or a hook
   failed), run `merge --abort` when `MERGE_HEAD` exists, check that `HEAD` equals the recorded
   commit, and report git's message.

**Rationale**: `merge-tree --write-tree` (git 2.38+) computes the merge in the object database only,
so a conflict leaves both working folders byte-identical, which a merge-then-abort cannot promise
for untracked or ignored files. A merge commit is Clarification Q2.

**Alternatives considered**: merge then abort (touches files); `--squash` and rebase (rejected in
Q2).

## R7. Removal guard

**Decision**: `core::lanes::guard::check_removable(root, ws, lane, worktrees)` passes only when
all hold:

1. The recorded path is absolute and has no `..` component.
2. The path's parent is exactly `<data root>/lanes/<workspace id>` (component comparison, not
   string prefix, so `lanes/<id>-evil` fails) and its last component equals the lane slug.
3. `symlink_metadata` says it is a directory, not a symbolic link; the canonical path equals the
   recorded path joined to the canonical data root.
4. `git -C <main checkout> worktree list --porcelain -z` lists exactly this path.

Removal then runs `git worktree remove [--force] <path>` (git deletes the folder; Sonata never
calls `remove_dir_all` on a checkout), then `git worktree prune`, then removes
`<data root>/lanes/<workspace id>` with `fs::remove_dir` if empty. `--force` is used only after
the second confirmation (FR-014). If the folder is already gone, only `prune` runs and the record
is cleared (FR-020 "Forget lane").

**Rationale**: Constitution V and FR-016. Letting git delete the folder means a checkout that git
does not recognise can never be deleted.

## R8. Branch deletion

**Decision**: Opt-in. `git branch -d lane/<slug>` first. If git refuses (not merged), the dialog
already showed the count from `rev-list --count <lane> --not --exclude=<lane> --branches` (commits on
no other branch; `--exclude` must come before `--branches`) and the user confirmed loss, so `branch -D` runs. Never for the base branch.

## R9. Human confirmation for requests from the CLI

**Decision**: A `Confirmations` registry in `core::lanes::confirm` holds pending requests
(`id → sender`). A CLI land or remove request builds the same preview the UI uses (R10), emits
`lane://confirm` with it, and waits on the receiver for up to 5 minutes, sending a progress line
every 15 s like `ask`. The UI shows `LaneDialog` and answers with `lane_confirm_answer`. No answer,
a closed window, or a decline returns exit code 3 (denied) and changes nothing. Only one pending
request per lane; a second one is refused.

Permissions (server side, FR-022): a terminal may request its own lane; an Orchestrator may request
the lane of a terminal linked to it. `sonata dismiss` of a terminal with a lane goes through the
same confirmation for the removal (FR-017).

**Rationale**: Clarification Q3; Constitution V (human confirms destructive operations, server
decides permissions).

## R10. Previews for dialogs

**Decision**: `lane_preview(node, action)` returns everything a dialog shows: branch, base, commits
to land, changed files in the lane (first 20 and total), commits only on the lane branch (for
branch deletion), and blocking reasons (dirty main checkout, wrong branch, operation in progress).
The UI and the CLI path use the same function, so both dialogs say the same thing.

## R11. Status polling

**Decision**: A core thread checks every 3 s the lanes of loaded workspaces whose terminals exist:
`status --porcelain=v2 -z` in the checkout (changed count; `GIT_OPTIONAL_LOCKS=0`), and
`rev-list --left-right --count <base>...<lane>` in the main checkout (ahead, behind). Results are
cached per node; `lane://status` is emitted only on change. A failed check marks the lane broken
(`checkoutMissing`, `branchMissing`) without deleting anything (FR-020). After create, land,
remove, and terminal exit, the node is checked at once.

**Rationale**: SC-007 (5 s). File watchers on large checkouts cost more than one cheap status call
every 3 s per lane, and agents rarely run more than a handful of lanes.

## R12. Agent instructions

**Decision**: `SessionInfo` gains an optional lane (branch, base). `instructions()` adds a "You
work in a lane" section: the branch, the base, "commit your work here; do not switch branches",
and the `sonata lane status|land` commands. `typed_instructions()` adds one sentence with the
branch and "commit your work; run `sonata lane land` when done".

## R13. Schema version

**Decision**: Workspace schema 2 → 3 with a no-op migration step. `TerminalContent` gains an
optional `lane` (omitted when empty).

**Rationale**: The new field is optional, so old files load unchanged. The bump makes an older
Sonata build refuse a v3 file ("written by a newer version") instead of loading it, dropping the
lane on its next save, and leaving checkouts that no workspace references.

## R14. Deleting a terminal or a workspace with lanes

**Decision**: `delete_node` on a terminal with an active lane requires a `LaneRemoval` (discard
changes, delete branch) chosen in the dialog; without it, it returns `LaneConfirmationNeeded`.
`delete_workspace` checks every lane first: if any has changes, it fails with the list; otherwise
it removes each lane (R7) before the workspace files. Landed lanes have no checkout and need
nothing.
