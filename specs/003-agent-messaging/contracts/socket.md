# Contract: CLI ↔ App Socket Protocol (v1)

Unix socket, path in `SONATA_SOCKET`. One request per connection. UTF-8 JSON, one object per line.

## Request (first line from the CLI)

```json
{"v": 1, "terminalId": "uuid", "token": "hex", "cwd": "/abs/path", "args": ["ask", "Backend", "hi"], "stdin": null}
```

## Responses (lines from the app)

```json
{"type": "progress", "message": "Backend is working (2m 10s)…"}
{"type": "result", "ok": true, "exitCode": 0, "text": "…", "data": {}}
{"type": "result", "ok": false, "exitCode": 3, "text": "Only the Orchestrator can do this."}
```

The connection closes after `result`. A request with a missing or wrong token gets exit code 3
and is logged without the token.

## Limits

Request line ≤ 128 KB; message ≤ 100 KB; result text ≤ 1 MB (truncated with a note).
