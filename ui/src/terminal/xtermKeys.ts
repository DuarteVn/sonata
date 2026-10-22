// Key overrides for live terminals.

/** Shift+Enter sends Escape + Return (Meta+Enter), which agent CLIs read as "new line". */
export const SHIFT_ENTER_SEQUENCE = "\x1b\r";

/**
 * Cmd shortcuts for line editing, as in other macOS terminals: Cmd+Backspace deletes the line
 * before the cursor (Ctrl+U), Cmd+Left and Cmd+Right go to the line start and end (Ctrl+A, Ctrl+E).
 */
const CMD_KEYS: Record<string, string> = {
  Backspace: "\x15",
  ArrowLeft: "\x01",
  ArrowRight: "\x05",
};

type KeyLike = Pick<KeyboardEvent, "key" | "shiftKey" | "altKey" | "ctrlKey" | "metaKey">;

/** Bytes to send instead of xterm's default for this key, or null to let xterm handle it. */
export function keyOverride(ev: KeyLike): string | null {
  if (ev.key === "Enter" && ev.shiftKey && !ev.altKey && !ev.ctrlKey && !ev.metaKey) return SHIFT_ENTER_SEQUENCE;
  if (ev.metaKey && !ev.shiftKey && !ev.altKey && !ev.ctrlKey) return CMD_KEYS[ev.key] ?? null;
  return null;
}
