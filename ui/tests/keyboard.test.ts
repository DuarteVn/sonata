import { describe, expect, it } from "vitest";
import { routeKey } from "../src/canvas/keyboard";

const free = { textFocus: false, terminalFocus: false, hasSelection: true, linking: false };

describe("canvas keyboard (FR-002, FR-004, FR-005)", () => {
  it.each([
    ["v", "tool:select"],
    ["t", "tool:terminal"],
    ["n", "tool:note"],
    ["Delete", "delete"],
    ["Backspace", "delete"],
    ["Escape", "clear"],
  ])("%s → %s without focus", (key, action) => {
    expect(routeKey({ key, ...free })).toBe(action);
  });

  it("ignores everything while a terminal or text field has focus", () => {
    for (const key of ["v", "t", "n", "Delete", "Backspace", "Escape"]) {
      expect(routeKey({ key, ...free, terminalFocus: true })).toBeNull();
      expect(routeKey({ key, ...free, textFocus: true })).toBeNull();
    }
  });

  it("delete needs a selection; escape cancels a pending link first", () => {
    expect(routeKey({ key: "Delete", ...free, hasSelection: false })).toBeNull();
    expect(routeKey({ key: "Escape", ...free, linking: true })).toBe("cancelLink");
  });

  it("ignores keys with modifiers", () => {
    expect(routeKey({ key: "t", ...free, meta: true })).toBeNull();
  });
});
