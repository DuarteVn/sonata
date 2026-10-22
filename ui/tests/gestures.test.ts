import { describe, expect, it } from "vitest";
import { routePointerDown, routeWheel, type Target } from "../src/canvas/gestures";

const focusedTerminal: Target = { kind: "terminal", id: "t1" };
const otherTerminal: Target = { kind: "terminal", id: "t2" };
const note: Target = { kind: "note", id: "n1" };
const empty: Target = { kind: "empty" };

describe("wheel routing (FR-008a)", () => {
  it("scrolls the focused terminal", () => {
    expect(routeWheel({ ctrlKey: false, deltaX: 0, deltaY: 30, target: focusedTerminal, focusedId: "t1" })).toEqual({
      type: "scrollTerminal",
      id: "t1",
    });
  });

  it.each([
    ["unfocused terminal", otherTerminal],
    ["note", note],
    ["empty canvas", empty],
  ])("pans over %s", (_, target) => {
    expect(routeWheel({ ctrlKey: false, deltaX: 5, deltaY: 30, target, focusedId: "t1" })).toEqual({
      type: "pan",
      dx: -5,
      dy: -30,
    });
  });

  it("pinch (ctrl+wheel) zooms even over the focused terminal", () => {
    const action = routeWheel({ ctrlKey: true, deltaX: 0, deltaY: -10, target: focusedTerminal, focusedId: "t1" });
    expect(action.type).toBe("zoom");
    if (action.type === "zoom") expect(action.factor).toBeGreaterThan(1);
  });
});

describe("pointer routing (FR-008a)", () => {
  it("middle button pans anywhere, even with focus", () => {
    expect(routePointerDown({ button: 1, spaceHeld: false, keyboardFocus: true, target: focusedTerminal })).toBe("pan");
  });

  it("space+drag pans only when nothing has keyboard focus", () => {
    expect(routePointerDown({ button: 0, spaceHeld: true, keyboardFocus: false, target: otherTerminal })).toBe("pan");
    expect(routePointerDown({ button: 0, spaceHeld: true, keyboardFocus: true, target: empty })).not.toBe("pan");
  });

  it("click on empty canvas clears focus", () => {
    expect(routePointerDown({ button: 0, spaceHeld: false, keyboardFocus: true, target: empty })).toBe("clearFocus");
  });

  it("click on a node is left to the node", () => {
    expect(routePointerDown({ button: 0, spaceHeld: false, keyboardFocus: false, target: note })).toBe("none");
  });
});
