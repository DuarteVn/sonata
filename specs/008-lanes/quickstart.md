# Quickstart: Validate Lanes

All scenarios use throwaway data and a throwaway repository. Never use a real project.

## Setup

```sh
export SANDBOX="$(mktemp -d)"
git init -q -b main "$SANDBOX/app" && cd "$SANDBOX/app"
printf 'one\n' > a.txt && mkdir web && printf 'x\n' > web/b.txt
git add . && git commit -qm init
SONATA_HOME="$SANDBOX/sonata" npm run tauri dev     # from the Sonata repo
```

Create a workspace whose folder is `$SANDBOX/app`.

## Automated

```sh
cargo test -p sonata-core --test lanes_git --test lanes_guard --test lanes_land --test lanes_cli
npm test -- lane
sh scripts/check.sh
```

## Scenario A — Two lanes in parallel (US1, SC-002)

1. New terminal "Alpha" with "Run in a lane" on; New terminal "Beta" the same way.
2. In Alpha: `printf 'alpha\n' > a.txt && git commit -qam alpha`. In Beta:
   `printf 'beta\n' > a.txt && git commit -qam beta`.
3. Expect: `cat $SANDBOX/app/a.txt` prints `one`; each lane prints its own text;
   `git -C $SANDBOX/app branch` lists `lane/alpha` and `lane/beta`; each node shows its branch and
   `↑1`.
4. Expect: the folders are under `$SANDBOX/sonata/lanes/<workspace id>/`, and
   `ls -l` shows normal file modes in the lanes after an app restart (research R4).

## Scenario B — Land and conflict (US2, SC-003)

1. Land Alpha from its menu, confirm. Expect: `a.txt` in the workspace folder is `alpha`;
   `git log --oneline -1` shows `Merge lane lane/alpha into main`; the Alpha node is marked landed;
   its folder is gone; `lane/alpha` still exists.
2. Land Beta. Expect: the dialog runs, then reports a conflict in `a.txt`;
   `git -C $SANDBOX/app status --porcelain` is empty and `HEAD` is unchanged.
3. Leave an uncommitted file in Beta and choose Land. Expect: refused, names the lane side.

## Scenario C — Remove and safety (US3, SC-004)

1. In Beta, create an untracked file. Remove lane: expect the file list and a second confirmation.
   Confirm: folder gone, `git worktree list` no longer shows it, `lane/beta` kept, the terminal
   restarts in `$SANDBOX/app`.
2. Quit the app, edit the workspace file so a lane's `checkoutPath` points at `$SANDBOX/app`,
   start the app, remove that lane. Expect: refused with an unsafe-path message; nothing deleted.

## Scenario D — Agent requests (US5)

1. Turn on Orchestrator for a terminal in the workspace folder; run
   `sonata recruit shell --name Gamma --lane`. Expect a new linked terminal in `lane/gamma`.
2. In Gamma: commit a change, run `sonata lane status` (expect `1 ahead`), then
   `sonata lane land`. Expect the app dialog; decline → exit 3 and nothing changed; run again and
   accept → merged, Gamma marked landed after the command printed the result.

## Scenario E — Not a repository

Create a workspace on an empty folder. Expect the lane checkbox disabled with the reason.
