# Contract: `sonata browser` Commands

All require the browser to be linked to the calling terminal. `<browser>`: title or short id from
`sonata list` (`b-xxxx`).

```text
sonata browser open <browser> <url> [--timeout 30s]
sonata browser read <browser>
sonata browser click <browser> <number|css> [--timeout 10s]
sonata browser fill <browser> <number|css> "<text>"
sonata browser press <browser> <Enter|Tab|Escape|ArrowUp|ArrowDown|…>
sonata browser screenshot <browser> [--out file.png]
sonata browser back|forward|reload <browser>
sonata browser eval <browser> "<js>"        # Orchestrator only
```

`read` output:

```text
Title: Login · My App
URL: http://localhost:3000/login

Welcome back
Sign in to continue

Interactive:
  [1] input  "Email"
  [2] input  "Password" (password)
  [3] button "Sign in"
  [4] link   "Forgot password?" → /reset
```
