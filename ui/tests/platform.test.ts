import { describe, expect, it } from "vitest";
import { isCloseItemKey, revealLabel, shortcut } from "../src/platform";

describe("platform labels", () => {
  it("writes shortcut hints the macOS way on a Mac", () => {
    expect(shortcut("toggleSidebar", true)).toBe("⌃⌘S");
    expect(shortcut("closeItem", true)).toBe("⌘W");
    expect(shortcut("settings", true)).toBe("⌘,");
  });

  it("uses Ctrl+Shift on Linux where Ctrl alone belongs to the terminal", () => {
    expect(shortcut("toggleSidebar", false)).toBe("Ctrl+Shift+S");
    expect(shortcut("closeItem", false)).toBe("Ctrl+Shift+W");
    expect(shortcut("quit", false)).toBe("Ctrl+Shift+Q");
    expect(shortcut("settings", false)).toBe("Ctrl+,");
  });

  it("names the file manager of each system", () => {
    expect(revealLabel(true)).toBe("Reveal in Finder");
    expect(revealLabel(false)).toBe("Show in File Manager");
  });

  it("names File Explorer on Windows (feature 012)", () => {
    expect(revealLabel(false, true)).toBe("Show in File Explorer");
  });

  it("closes the item with plain Ctrl+W on Windows only, outside terminals and fields", () => {
    const key = (k: string, shiftKey = false) => ({ key: k, ctrlKey: true, shiftKey, altKey: false, metaKey: false });
    const none = { terminal: false, text: false };
    expect(isCloseItemKey(key("w"), none, true)).toBe(true);
    expect(isCloseItemKey(key("W"), none, true)).toBe(true);
    expect(isCloseItemKey(key("w"), { terminal: true, text: false }, true)).toBe(false);
    expect(isCloseItemKey(key("w"), { terminal: false, text: true }, true)).toBe(false);
    expect(isCloseItemKey(key("w", true), none, true)).toBe(false);
    expect(isCloseItemKey(key("w"), none, false)).toBe(false);
  });
});
