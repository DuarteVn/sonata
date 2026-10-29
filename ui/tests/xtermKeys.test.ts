import { describe, expect, it } from "vitest";
import { clipboardAction, keyOverride, SHIFT_ENTER_SEQUENCE } from "../src/terminal/xtermKeys";

const key = (k: string, mods: Partial<Record<"shiftKey" | "altKey" | "ctrlKey" | "metaKey", boolean>> = {}) => ({
  key: k,
  shiftKey: false,
  altKey: false,
  ctrlKey: false,
  metaKey: false,
  ...mods,
});

describe("terminal key overrides", () => {
  it("sends a new-line sequence for Shift+Enter", () => {
    expect(keyOverride(key("Enter", { shiftKey: true }))).toBe(SHIFT_ENTER_SEQUENCE);
  });

  it("leaves Enter, other modifiers, and typed characters to xterm", () => {
    expect(keyOverride(key("Enter"))).toBeNull();
    expect(keyOverride(key("Enter", { shiftKey: true, metaKey: true }))).toBeNull();
    expect(keyOverride(key("ç"))).toBeNull();
    expect(keyOverride(key("ç", { altKey: true }))).toBeNull();
  });

  it("maps Cmd line-editing shortcuts to the shell's control keys on a Mac", () => {
    expect(keyOverride(key("Backspace", { metaKey: true }), true)).toBe("\x15");
    expect(keyOverride(key("ArrowLeft", { metaKey: true }), true)).toBe("\x01");
    expect(keyOverride(key("ArrowRight", { metaKey: true }), true)).toBe("\x05");
    expect(keyOverride(key("Backspace"), true)).toBeNull();
    expect(keyOverride(key("c", { metaKey: true }), true)).toBeNull();
  });

  it("leaves the Super key alone on Linux", () => {
    expect(keyOverride(key("Backspace", { metaKey: true }), false)).toBeNull();
    expect(keyOverride(key("Enter", { shiftKey: true }), false)).toBe(SHIFT_ENTER_SEQUENCE);
  });
});

describe("terminal clipboard keys", () => {
  it("copies and pastes with Ctrl+Shift+C and Ctrl+Shift+V on Linux", () => {
    expect(clipboardAction(key("C", { ctrlKey: true, shiftKey: true }), false)).toBe("copy");
    expect(clipboardAction(key("V", { ctrlKey: true, shiftKey: true }), false)).toBe("paste");
  });

  it("leaves Ctrl+C, Ctrl+V, and the Mac to the terminal and the Edit menu", () => {
    expect(clipboardAction(key("c", { ctrlKey: true }), false)).toBeNull();
    expect(clipboardAction(key("v", { ctrlKey: true }), false)).toBeNull();
    expect(clipboardAction(key("C", { ctrlKey: true, shiftKey: true }), true)).toBeNull();
  });
});
