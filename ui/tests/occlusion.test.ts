import { describe, expect, it } from "vitest";
import { occlusionClip } from "../src/terminal/occlusion";

const body = { left: 100, top: 100, width: 200, height: 100 };

/** Area left visible by a clip, from its `M x,y h w v h …z` rectangles. */
function visibleArea(clip: string) {
  return [...clip.matchAll(/h(-?[\d.]+)v(-?[\d.]+)/g)]
    .map(([, w, h]) => Number(w) * Number(h))
    .reduce((a, b) => a + b, 0);
}

describe("live terminal occlusion", () => {
  it("does not clip an uncovered view", () => {
    expect(occlusionClip(body, [])).toBeUndefined();
    expect(occlusionClip(body, [{ left: 400, top: 100, width: 50, height: 50 }])).toBeUndefined();
  });

  it("cuts out the part under a higher node", () => {
    const clip = occlusionClip(body, [{ left: 250, top: 50, width: 100, height: 100 }])!;
    expect(clip.startsWith('path("')).toBe(true);
    expect(visibleArea(clip)).toBe(200 * 100 - 50 * 50);
  });

  it("handles overlapping covers without double counting", () => {
    const clip = occlusionClip(body, [
      { left: 100, top: 100, width: 100, height: 100 },
      { left: 150, top: 100, width: 100, height: 100 },
    ])!;
    expect(visibleArea(clip)).toBe(50 * 100);
  });

  it("hides a fully covered view", () => {
    expect(occlusionClip(body, [{ left: 0, top: 0, width: 1000, height: 1000 }])).toBe("inset(50%)");
  });

  it("uses coordinates local to the view", () => {
    expect(occlusionClip(body, [{ left: 100, top: 150, width: 200, height: 50 }])).toBe('path("M0,0h200v50h-200z")');
  });
});
