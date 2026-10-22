# Contract: `sonata` Command-Line Tool

Available only inside Sonata terminals. `--json` on any command prints machine-readable output.
Exit codes: 0 ok, 1 other error, 2 usage, 3 not allowed, 4 not found, 5 timed out,
6 Sonata not reachable.

## Everyone

```text
sonata list
sonata ask <name>[,<name>…] "<message>" [--timeout 10m]
sonata ask --all "<message>" [--timeout 10m]
sonata ask <name> --keys "<keys>"          # enter tab esc up down left right space backspace ctrl-c ctrl-d "text"
sonata check <name>
sonata reply "<message>"                   # or: sonata reply --stdin
sonata note read <note>
sonata note write <note> "<text>" [--append]   # or --stdin
sonata notify "<message>"
sonata run <claude|codex> [agent args…]     # used by presets; adds session instructions and hooks
sonata help [command]
```

`<name>`: a linked terminal's name (case-insensitive). `<note>`: a linked note's title or the
short id shown by `sonata list`.

## Orchestrator only

```text
sonata recruit <claude|codex|opencode|gemini|shell|"<command>"> [--name N] [--role R | --role-text T] [--folder F]
sonata dismiss <name>
sonata link <a> <b>
sonata unlink <a> <b>
```

## Internal (called by agent hooks, not documented to agents)

```text
sonata _hook claude-stop          # stdin: Claude Code Stop hook JSON
sonata _hook turn-start           # stdin: Claude Code UserPromptSubmit hook JSON
sonata _hook codex-notify <json>  # Codex notify payload
```

## Output examples

```text
$ sonata list
You are "Lead" in workspace "Sonata" · Orchestrator
Role: You coordinate the workers.
Linked:
  terminal  Backend   working   codex
  terminal  Reviewer  idle      claude
  note      Plan      (n-3f2a)  "# Release plan"

$ sonata ask Backend "Which endpoints lack tests?"
[Backend answered in 41 s]
The following endpoints have no tests: …
```
