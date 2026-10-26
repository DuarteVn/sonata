# Quickstart: Validating Session Restore

Run a dev build next to the installed Sonata with throwaway data:

```sh
export SANDBOX="$(mktemp -d)"
cd /path/to/sonata && SONATA_HOME="$SANDBOX/sonata" \
  npm run tauri dev -- --config '{"identifier":"dev.sonata.app.test009"}'
```

Launch it from a Claude Code terminal on purpose: that is the environment that used to leak
session markers (scenario B).

## A. Restore after reopen (US1)

1. Create a workspace and a terminal with launch command `claude`.
2. Send: "Remember the word PELICAN. Reply OK."
3. Quit the dev app (Cmd+Q) and start it again with the same command.
4. The terminal starts `claude --resume …` by itself. Ask: "Which word did I give you?"
   Expected: PELICAN.
5. Check `$SANDBOX/sonata/workspaces/*/workspace.json`: the terminal has `conversation`.

## B. Clean environment (US2)

1. In any terminal of the dev app run `env | grep -E '^(CLAUDECODE|CLAUDE_CODE_|CLAUDE_PID)'`.
   Expected: no output.
2. Start `claude`. Expected: no "Transcript saving is off" warning.

## C. Conversation follows the agent (US3)

1. In the scenario A terminal type `/clear`, then "Remember OTTER. Reply OK."
2. Quit and reopen. Ask for the word. Expected: OTTER.
3. Type `/resume`, pick the PELICAN conversation, quit and reopen. Expected: PELICAN.

## D. Start fresh (US4)

1. Terminal menu → Start fresh. Expected: agent restarts with an empty conversation; `conversation`
   disappears from the workspace file.
2. While the agent is replying, choose Start fresh again. Expected: confirmation dialog.

## E. Resume fails (US5)

1. Quit the dev app. Delete the stored conversation's file:
   `rm ~/.claude/projects/*/<id>.jsonl` (id from the workspace file).
2. Start the app. Expected: one line
   `sonata: the previous conversation could not be resumed; starting a new one.`, then a fresh
   agent ready for input. Quit and reopen again: no notice.

## F. Codex (when installed)

Repeat A and E with launch command `codex`. Also confirm that `thread-id` from the notify payload
is accepted by `codex resume <id>` (research R2).

## Automated gate

`sh scripts/check.sh` passes.

## Results (2026-10-26, dev build `dev.sonata.app.test009`, throwaway `SONATA_HOME`)

Claude Code v2.1.310. The dev app was started from a Claude Code terminal (all R5 markers set).
Messages were sent with `sonata ask` from a linked shell terminal. "Quit" was SIGTERM to the
dev process instead of Cmd+Q: the conversation is saved when it changes, not at quit.

- **A**: pass. After reopen the terminal ran `claude --resume <id> --append-system-prompt-file …`
  with the old messages on screen and answered PELICAN. `workspace.json` held `conversation`
  right after the first message.
- **B**: pass. No `CLAUDE*` or `CODEX*` variable in a terminal. Claude started without the
  "Transcript saving is off" warning.
- **C**: pass. `/clear` removed `conversation`, and the first message stored the new id; reopen
  answered OTTER. `/resume` → PELICAN put the old id back; reopen answered PELICAN.
- **D**: not run. It needs clicks in the terminal menu, and UI scripting has no Accessibility
  permission here. Covered by core and UI tests (T025, T029).
- **E**: partial. With the conversation file moved away, the app started a new conversation by
  itself (no `--resume` in the new process, `conversation` cleared), and the next reopen tried
  nothing. The notice line was **not visible**: Claude Code clears the screen when it starts,
  which removes the line the wrapper printed just before. Open question: show the notice
  another way.
