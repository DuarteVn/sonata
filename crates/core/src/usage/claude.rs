//! Claude Code provider: login status and plan limits read through the `claude` CLI (research R1).

/// Probe arguments (research R1): one tiny print-mode request whose `stream-json` output
/// carries a `rate_limit_event`. Shown verbatim in the consent dialog.
pub const PROBE_ARGS: [&str; 18] = [
    "-p",
    ".",
    "--model",
    "haiku",
    "--system-prompt",
    "Reply with one dot.",
    "--tools",
    "",
    "--setting-sources",
    "",
    "--strict-mcp-config",
    "--disable-slash-commands",
    "--no-session-persistence",
    "--max-turns",
    "1",
    "--output-format",
    "stream-json",
    "--verbose",
];

/// Reads Claude Code usage. Filled in by T031 (US1).
pub fn read(
    _path: &std::path::Path,
    _env: &[(String, String)],
    now: chrono::DateTime<chrono::Utc>,
) -> super::Reading {
    super::Reading::unavailable("claude", now, "Not supported yet.")
}
