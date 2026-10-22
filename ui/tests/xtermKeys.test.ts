import { describe, expect, it } from "vitest";
import { keyOverride, SHIFT_ENTER_SEQUENCE } from "../src/terminal/xtermKeys";

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

  it("maps Cmd line-editing shortcuts to the shell's control keys", () => {
    expect(keyOverride(key("Backspace", { metaKey: true }))).toBe("\x15");
    expect(keyOverride(key("ArrowLeft", { metaKey: true }))).toBe("\x01");
    expect(keyOverride(key("ArrowRight", { metaKey: true }))).toBe("\x05");
    expect(keyOverride(key("Backspace"))).toBeNull();
    expect(keyOverride(key("c", { metaKey: true }))).toBeNull();
  });
});
