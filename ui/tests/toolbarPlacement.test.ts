import { describe, expect, it } from "vitest";
import { placeToolbar } from "../src/canvas/NodeToolbar";

const viewport = { left: 0, top: 0, width: 1000, height: 700 };
const size = { width: 300, height: 40 };

describe("node toolbar placement", () => {
  it("centers above the node", () => {
    const p = placeToolbar({ left: 400, top: 300, width: 200, height: 100 }, viewport, size);
    expect(p).toEqual({ left: 350, top: 300 - 40 - 10, below: false });
  });

  it("goes below when there is little room above", () => {
    const p = placeToolbar({ left: 400, top: 30, width: 200, height: 100 }, viewport, size)!;
    expect(p.below).toBe(true);
    expect(p.top).toBe(30 + 100 + 10);
  });

  it("clamps horizontally inside the viewport", () => {
    expect(placeToolbar({ left: -100, top: 300, width: 200, height: 100 }, viewport, size)!.left).toBe(8);
    expect(placeToolbar({ left: 950, top: 300, width: 200, height: 100 }, viewport, size)!.left).toBe(1000 - 300 - 8);
  });

  it("is hidden when the node is off screen", () => {
    expect(placeToolbar({ left: 2000, top: 300, width: 200, height: 100 }, viewport, size)).toBeNull();
  });
});
