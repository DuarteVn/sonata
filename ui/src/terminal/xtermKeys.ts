// Key overrides for live terminals.

/** Shift+Enter sends Escape + Return (Meta+Enter), which agent CLIs read as "new line". */
export const SHIFT_ENTER_SEQUENCE = "\x1b\r";

type KeyLike = Pick<KeyboardEvent, "key" | "shiftKey" | "altKey" | "ctrlKey" | "metaKey">;

/** Bytes to send instead of xterm's default for this key, or null to let xterm handle it. */
export function keyOverride(ev: KeyLike): string | null {
  if (ev.key === "Enter" && ev.shiftKey && !ev.altKey && !ev.ctrlKey && !ev.metaKey) return SHIFT_ENTER_SEQUENCE;
  return null;
}
