# Contract: `sonata lane` and `sonata recruit --lane`

Exit codes as in feature 003: 0 ok, 1 error, 2 usage, 3 denied (includes a human decline or no
answer), 4 not found, 5 timeout.

## `sonata recruit … --lane` (Orchestrator)

Adds `--lane` to the existing command. The new terminal is created in its own lane from the
workspace folder's current branch. `--lane` and `--folder` cannot be combined (exit 2).

```text
$ sonata recruit claude --name "Login form" --lane
Recruited "Login form" in lane lane/login-form (base main). It is linked to you; ask it with `sonata ask "Login form" "…"`.
```

Errors (exit 1): `The workspace folder is not inside a git repository.`, `The workspace folder has
no branch checked out (detached HEAD).`, `A merge, rebase, or cherry-pick is in progress in the
workspace folder.`, `Git is not installed. …`.

## `sonata lane status [<terminal>]`

No argument: the caller's own lane. With `<terminal>`: a terminal linked to the caller.

```text
$ sonata lane status
Lane lane/login-form (base main): 2 ahead, 0 behind, 3 changed files.
```

`data`: `{"branch","baseBranch","state","changed","ahead","behind"}`. No lane: exit 4,
`This terminal has no lane.`

## `sonata lane land [<terminal>] [--timeout 5m]`

No argument: own lane. With `<terminal>`: Orchestrator only, the terminal must be linked to the
caller (else exit 3). The app shows the land confirmation; the command waits (progress line every
15 s) for at most `--timeout` (1m–30m, default 5m).

```text
$ sonata lane land
Waiting for the user to confirm the land in Sonata…
Landed lane/login-form into main (2 commits). The lane was removed; the branch is kept.
```

- Declined: exit 3, `The user declined the land. Nothing changed.`
- No answer: exit 3, `No answer from the user within 5m. Nothing changed.`
- Refused before asking (exit 1): the blocking reason from the preview, for example `The lane has
  3 uncommitted files. Commit them first.` or `Nothing to land: main already has every commit.`
- Conflict (exit 1): `Land cancelled: conflicts in src/a.ts, src/b.ts and 4 more. Nothing
  changed. Merge main into your lane, resolve, commit, and land again.`

When the caller lands its own lane, the reply is sent before its process is stopped (FR-011).

## `sonata lane remove [<terminal>] [--delete-branch] [--timeout 5m]`

Same permissions and waiting as `land`. `--delete-branch` pre-selects the branch option in the
dialog; the human can still change it. Uncommitted changes are shown in the dialog with the
discard confirmation.

```text
Removed lane lane/login-form. The branch is kept.
```

## `sonata dismiss <name>` (changed)

For a recruited terminal that has an active lane, the app first shows the remove-lane
confirmation. Declined or no answer: exit 3, nothing is dismissed.

## Help

`sonata help` lists `lane status|land|remove`. `sonata help lane` shows the forms above.
