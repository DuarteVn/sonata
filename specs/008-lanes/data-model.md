# Data Model: Lanes

**Feature**: 008-lanes | **Date**: 2026-10-23

## Stored: `TerminalContent.lane` (workspace file, schema 3)

```json
"lane": {
  "branch": "lane/login-form",
  "baseBranch": "main",
  "mainCheckout": "/Users/me/Developer/app",
  "checkoutPath": "/Users/me/.sonata/lanes/6f1c…/login-form",
  "subdirectory": "web",
  "createdAt": "2026-10-23T12:00:00Z",
  "landedAt": "2026-10-23T15:30:00Z"
}
```

| Field | Rule |
|---|---|
| `branch` | `lane/<slug>`, valid for `git check-ref-format --branch`; unique when created. |
| `baseBranch` | Short name of the branch checked out in the workspace folder at creation. |
| `mainCheckout` | Absolute top folder of the workspace's repository; land runs here. |
| `checkoutPath` | Absolute; `<data root>/lanes/<workspace id>/<slug>`; must pass the removal guard (research R7) before any delete. |
| `subdirectory` | Workspace folder relative to `mainCheckout`; omitted when empty; no `..`. |
| `createdAt` | Creation time. |
| `landedAt` | Set by a successful land; the checkout no longer exists. Omitted otherwise. |

Rules:

- One lane per terminal; set only at creation (FR-007). Edit terminal never changes it.
- Terminal working folder while the lane is active: `checkoutPath/subdirectory`. The terminal's own
  `workingDirectory` override is not used while a lane is set.
- Restarting a landed terminal clears `lane`; it then runs in the workspace folder.
- "Forget lane" clears `lane` without touching git (only when the checkout is missing).

## Derived, never stored: `LaneStatus` (event `lane://status`)

```json
{ "nodeId": "…", "state": "ok", "branch": "lane/login-form", "baseBranch": "main",
  "changed": 3, "ahead": 2, "behind": 0 }
```

`state`: `ok` | `landed` | `checkoutMissing` | `branchMissing` | `mainCheckoutMissing` |
`busy` (a merge, rebase, or cherry-pick is in progress in the lane) | `gitUnavailable`.
Counts are absent when the state is not `ok` or `busy`.

## State transitions

```text
(create, checkbox on) ──► active ──land ok──► landed ──restart──► (no lane)
                            │  ▲                 └──delete node──► (gone)
                            │  └─ status poll: ok / busy
                            ├──remove──► (no lane, terminal in workspace folder)
                            ├──checkout or branch vanished──► broken ──forget──► (no lane)
                            └──delete node──► remove rules ──► (gone)
```

## Not stored

- Confirmation requests (in memory, research R9).
- The per-repository land lock.
- Git availability (checked once per run).

## Migration

`workspace` schema 2 → 3: no payload change (`lane` is optional). See research R13.
