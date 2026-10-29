import { describe, expect, it } from "vitest";
import { revealLabel, shortcut } from "../src/platform";

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
});
