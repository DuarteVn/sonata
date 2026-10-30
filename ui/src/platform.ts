// What differs between macOS, Linux, and Windows in the interface (feature 011, research R11;
// feature 012, research R15). Windows follows Linux except where noted.

const userAgent = typeof navigator !== "undefined" ? navigator.userAgent : "";

/** True in the macOS app; false on Linux (WebKitGTK reports "X11; Linux") and Windows. */
export const isMac = /Macintosh|Mac OS X/.test(userAgent);

/** True in the Windows app (WebView2 reports "Windows NT"). */
export const isWindows = /Windows/.test(userAgent);

export type ShortcutName = "toggleSidebar" | "closeItem" | "settings" | "quit";

/** Shortcut hints as each OS writes them; Linux uses Ctrl+Shift where Ctrl alone is a terminal key. */
const SHORTCUTS: Record<ShortcutName, [mac: string, linux: string]> = {
  toggleSidebar: ["⌃⌘S", "Ctrl+Shift+S"],
  closeItem: ["⌘W", "Ctrl+Shift+W"],
  settings: ["⌘,", "Ctrl+,"],
  quit: ["⌘Q", "Ctrl+Shift+Q"],
};

export function shortcut(name: ShortcutName, mac = isMac): string {
  return SHORTCUTS[name][mac ? 0 : 1];
}

/** The action that shows a file in the system file manager. */
export function revealLabel(mac = isMac, windows = isWindows): string {
  if (mac) return "Reveal in Finder";
  return windows ? "Show in File Explorer" : "Show in File Manager";
}

type Keys = Pick<KeyboardEvent, "key" | "ctrlKey" | "shiftKey" | "altKey" | "metaKey">;

/**
 * Windows only: plain Ctrl+W also closes the selected item when no terminal or text field has
 * focus (they keep the key). Ctrl+Shift+W, the menu shortcut, works everywhere.
 */
export function isCloseItemKey(e: Keys, focus: { terminal: boolean; text: boolean }, windows = isWindows): boolean {
  return (
    windows &&
    e.ctrlKey &&
    !e.shiftKey &&
    !e.altKey &&
    !e.metaKey &&
    e.key.toLowerCase() === "w" &&
    !focus.terminal &&
    !focus.text
  );
}
