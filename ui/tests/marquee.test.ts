import { describe, expect, it } from "vitest";
import { nodesInRect, rectFromPoints } from "../src/canvas/marquee";
import type { CanvasNode } from "../src/ipc/types";

const node = (id: string, x: number, y: number, width = 100, height = 100) =>
  ({ id, frame: { x, y, width, height }, z: 0, content: { note: {} } }) as unknown as CanvasNode;

describe("selection rectangle", () => {
  it("builds the same rectangle from corners in any order", () => {
    const r = { left: 10, top: 20, width: 90, height: 60 };
    expect(rectFromPoints({ x: 10, y: 20 }, { x: 100, y: 80 })).toEqual(r);
    expect(rectFromPoints({ x: 100, y: 80 }, { x: 10, y: 20 })).toEqual(r);
  });

  it("selects every node the rectangle touches", () => {
    const nodes = [node("a", 0, 0), node("b", 150, 0), node("c", 400, 400)];
    expect(nodesInRect(nodes, rectFromPoints({ x: 50, y: 50 }, { x: 160, y: 60 }))).toEqual(["a", "b"]);
    expect(nodesInRect(nodes, rectFromPoints({ x: 110, y: 110 }, { x: 140, y: 140 }))).toEqual([]);
  });
});
