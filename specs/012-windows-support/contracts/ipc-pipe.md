# Contract: CLI channel on Windows

Same messages as `specs/003-agent-messaging/contracts` (newline-delimited JSON request with
`token`, one or more JSON lines back). Only the transport differs.

## Server (app)

- Name: `\\.\pipe\sonata-<16 lowercase hex>`; exported to terminals as `SONATA_SOCKET`.
- First instance created with `FILE_FLAG_FIRST_PIPE_INSTANCE`; if that fails because the name
  exists, the app logs an error and runs without the CLI channel (as when the Unix socket
  cannot bind).
- Open mode `PIPE_ACCESS_DUPLEX`; pipe mode `PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT |
  PIPE_REJECT_REMOTE_CLIENTS`; unlimited instances; one thread per connection, as today.
- Security descriptor (SDDL): `D:P(A;;GA;;;<user SID>)(A;;GA;;;SY)`. No other entries.
- Stop: the server sets its stop flag and connects once to unblock the waiting instance.

## Client (`sonata`)

- Reads `SONATA_SOCKET`; missing → "not inside a Sonata terminal" error, as today.
- Opens the pipe read/write. `ERROR_PIPE_BUSY` → `WaitNamedPipeW` up to 5 s, then retry once.
  `ERROR_FILE_NOT_FOUND` → "Sonata is not running" error, as today.
- Verifies `GetNamedPipeServerProcessId` belongs to a process whose token user SID equals the
  client's; otherwise refuses with "The Sonata channel is owned by another user." and sends
  nothing (the token is never written to a foreign server).

## Security tests

- DACL of a created pipe has exactly the two entries above and is protected.
- A request without a valid token gets the same `unauthorized` error as on macOS.
- A client run with a different `SONATA_SOCKET` pipe created by the test under a fake owner
  check refuses to send.
