// Key overrides for live terminals.

import { isMac, isWindows } from "../platform";

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
export function keyOverride(ev: KeyLike, mac = isMac): string | null {
  if (ev.key === "Enter" && ev.shiftKey && !ev.altKey && !ev.ctrlKey && !ev.metaKey) return SHIFT_ENTER_SEQUENCE;
  if (mac && ev.metaKey && !ev.shiftKey && !ev.altKey && !ev.ctrlKey) return CMD_KEYS[ev.key] ?? null;
  return null;
}

/**
 * Linux terminals copy and paste with Ctrl+Shift+C and Ctrl+Shift+V, because Ctrl+C and Ctrl+V
 * belong to the program in the terminal. On a Mac the Edit menu does this with Cmd+C and Cmd+V.
 * Windows adds its terminals' habit (feature 012): Ctrl+C copies when text is selected (and
 * otherwise reaches the program), and Ctrl+V pastes.
 */
export function clipboardAction(
  ev: KeyLike,
  mac = isMac,
  windows = isWindows,
  hasSelection = false,
): "copy" | "paste" | null {
  if (mac || !ev.ctrlKey || ev.altKey || ev.metaKey) return null;
  const k = ev.key.toLowerCase();
  if (ev.shiftKey) return k === "c" ? "copy" : k === "v" ? "paste" : null;
  if (!windows) return null;
  if (k === "c" && hasSelection) return "copy";
  return k === "v" ? "paste" : null;
}
