import { describe, expect, it } from "vitest";
import { computeLiveSet, WEBGL_BUDGET, type TerminalRect } from "../src/terminal/liveSet";

const viewport = { left: 0, top: 0, width: 1000, height: 800 };

function grid(n: number, size = 100): TerminalRect[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `t${i}`,
    rect: { left: (i % 8) * (size + 5), top: Math.floor(i / 8) * (size + 5), width: size, height: size },
  }));
}

describe("live set (research R3)", () => {
  it("is live only when visible and legible", () => {
    const terminals: TerminalRect[] = [
      { id: "in", rect: { left: 10, top: 10, width: 100, height: 100 } },
      { id: "out", rect: { left: 1100, top: 10, width: 100, height: 100 } },
      { id: "far", rect: { left: 5000, top: 10, width: 100, height: 100 } },
    ];
    const ok = computeLiveSet({ terminals, viewport, zoom: 1, baseFontSize: 13, focusedId: null, lost: new Set() });
    expect(ok.live.map((l) => l.id)).toEqual(["in"]);
    expect(ok.previews).toEqual(["out"]);

    const small = computeLiveSet({ terminals, viewport, zoom: 0.5, baseFontSize: 13, focusedId: null, lost: new Set() });
    expect(small.live).toEqual([]);
    expect(13 * 0.55).toBeGreaterThanOrEqual(7);
    const edge = computeLiveSet({ terminals, viewport, zoom: 0.55, baseFontSize: 13, focusedId: null, lost: new Set() });
    expect(edge.live.map((l) => l.id)).toEqual(["in"]);
  });

  it("uses WebGL for at most 12, focused first, DOM for the rest", () => {
    expect(WEBGL_BUDGET).toBe(12);
    const r = computeLiveSet({ terminals: grid(30, 90), viewport, zoom: 1, baseFontSize: 13, focusedId: "t29", lost: new Set() });
    const webgl = r.live.filter((l) => l.renderer === "webgl");
    expect(webgl).toHaveLength(12);
    expect(webgl.map((l) => l.id)).toContain("t29");
    expect(r.live.filter((l) => l.renderer === "dom").length).toBe(r.live.length - 12);
  });

  it("falls back to DOM after context loss", () => {
    const r = computeLiveSet({ terminals: grid(2), viewport, zoom: 1, baseFontSize: 13, focusedId: null, lost: new Set(["t0"]) });
    expect(r.live.find((l) => l.id === "t0")?.renderer).toBe("dom");
    expect(r.live.find((l) => l.id === "t1")?.renderer).toBe("webgl");
  });
});
