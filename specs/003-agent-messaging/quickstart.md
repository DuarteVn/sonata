# Quickstart: Validate Feature 003

Same build and test commands as 001. Use a throwaway `SONATA_HOME`.

1. **Ask between shells (US1)**: two Shell terminals A and B, linked. In A: `sonata ask B "echo
   hello"` → prints `hello`. Unlink → same command exits 3 and lists linked names.
2. **Parallel and keys**: link A to B and C; `sonata ask --all "pwd"` → two labeled answers.
   In B run `cat`; from A `sonata ask B --keys ctrl-c` → `cat` stops.
3. **Reply**: in B run `read x; sonata reply "got $x"`; from A `sonata ask B "hi"` → `got hi`.
4. **Agents (US2)**: create a Claude Code terminal from the preset with a role; ask it "what can
   you do in Sonata and who are you linked to?" → it describes `sonata` and its links. Check that
   `~/.claude/settings.json` is unchanged (`git diff --no-index` against a copy).
5. **Claude Code answers**: from a shell linked to it, `sonata ask Claude "Summarize README.md in
   3 bullets"` → returns the bullets when Claude finishes.
6. **Orchestrator (US3)**: in an Orchestrator shell, `sonata recruit claude --name Reviewer
   --role-text "Review diffs"` → new linked terminal; `sonata dismiss Reviewer` → gone. In a
   non-orchestrator → exit 3.
7. **Notes and notify (US4)**: link a note "# Plan"; `sonata note write Plan --append "- done"` →
   canvas updates; `sonata notify "finished"` → macOS notification.
8. **Visibility (US5)**: during an ask the link glows and the target shows "asked by A"; open the
   log from the terminal toolbar.
9. **Safety**: from A, set `SONATA_TOKEN=bad sonata list` → exit 3. Loop: B asks A while A waits
   on B → refused with the chain.
