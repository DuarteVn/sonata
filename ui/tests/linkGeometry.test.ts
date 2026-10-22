import { describe, expect, it } from "vitest";
import { linkAnchors, linkPath } from "../src/canvas/linkGeometry";

const box = (x: number, y: number, w = 100, h = 50) => ({ x, y, width: w, height: h });

describe("link geometry", () => {
  it("uses facing edges side by side", () => {
    const [p, q] = linkAnchors(box(0, 0), box(300, 0));
    expect(p).toEqual({ x: 100, y: 25, side: "right" });
    expect(q).toEqual({ x: 300, y: 25, side: "left" });
    const [r, s] = linkAnchors(box(300, 0), box(0, 0));
    expect(r.side).toBe("left");
    expect(s.side).toBe("right");
  });

  it("uses top/bottom when stacked", () => {
    const [p, q] = linkAnchors(box(0, 0), box(10, 300));
    expect(p).toEqual({ x: 50, y: 50, side: "bottom" });
    expect(q).toEqual({ x: 60, y: 300, side: "top" });
  });

  it("falls back to centers when overlapping", () => {
    const [p, q] = linkAnchors(box(0, 0), box(20, 10));
    expect(p.side).toBe("center");
    expect(q.side).toBe("center");
  });

  it("path starts and ends at the anchors", () => {
    const [p, q] = linkAnchors(box(0, 0), box(300, 0));
    const d = linkPath(p, q);
    expect(d.startsWith(`M ${p.x} ${p.y} C`)).toBe(true);
    expect(d.endsWith(`${q.x} ${q.y}`)).toBe(true);
  });
});
