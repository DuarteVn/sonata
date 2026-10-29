// What differs between macOS and Linux in the interface (feature 011, research R11).

/** True in the macOS app; false on Linux (WebKitGTK reports "X11; Linux"). */
export const isMac = typeof navigator !== "undefined" && /Macintosh|Mac OS X/.test(navigator.userAgent);

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
export function revealLabel(mac = isMac): string {
  return mac ? "Reveal in Finder" : "Show in File Manager";
}
