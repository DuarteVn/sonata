# Contract: shells and agent helpers on Windows

| Shell | Started as | Helpers (`claude`, `codex` → `sonata run …`) |
|---|---|---|
| PowerShell 7 / Windows PowerShell | `<exe> -NoLogo -NoExit -EncodedCommand <helpers[; launch]>` | yes, defined only when `sonata` is found |
| Git Bash | `bash.exe --rcfile <root>\run\bash\sonata.bashrc -i` | yes |
| Command Prompt | `%ComSpec%` (launch command: `/K <command>`) | no |
| WSL | `wsl.exe -d <distro> --cd <cwd>` (launch: `-- <command>` through the login shell) | no; `sonata.exe` reachable through `WSLENV` |

Environment for every Sonata terminal on Windows: `SONATA_SOCKET`, `SONATA_TOKEN`,
`SONATA_TERMINAL_ID`, `Path` with the CLI folder first (joined with `;`), `TERM=xterm-256color`.
The user's profile files, execution policy, and global agent settings are never written.

`sonata.bashrc` loads, in order: `/etc/profile`, then `~/.bash_profile` if it exists, else
`~/.bashrc`; then defines the helpers when `command -v sonata` succeeds.
