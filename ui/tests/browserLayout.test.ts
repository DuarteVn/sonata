import { describe, expect, it } from "vitest";
import { MAX_LIVE_BROWSERS, planBrowsers } from "../src/canvas/browserLayout";

const viewport = { left: 0, top: 0, width: 2000, height: 1200 };
const page = (id: string, left: number, width = 400) => ({ id, rect: { left, top: 100, width, height: 300 } });

describe("browser live set (004 FR-007)", () => {
  it("is live when visible, legible, not covered, and no gesture", () => {
    const r = planBrowsers({ pages: [page("a", 10)], viewport, zoom: 1, gesturing: false, covers: [], selectedId: null });
    expect(r).toEqual([{ id: "a", rect: { left: 10, top: 100, width: 400, height: 300 } }]);
  });

  it("shows cards during gestures, at small zoom, off screen, or when covered", () => {
    const base = { pages: [page("a", 10)], viewport, zoom: 1, gesturing: false, covers: [], selectedId: null };
    expect(planBrowsers({ ...base, gesturing: true })[0].rect).toBeNull();
    expect(planBrowsers({ ...base, zoom: 0.3 })[0].rect).toBeNull();
    expect(planBrowsers({ ...base, pages: [page("a", 5000)] })[0].rect).toBeNull();
    expect(planBrowsers({ ...base, covers: [{ left: 300, top: 200, width: 50, height: 50 }] })[0].rect).toBeNull();
    expect(planBrowsers({ ...base, covers: [{ left: 1500, top: 200, width: 50, height: 50 }] })[0].rect).not.toBeNull();
  });

  it("keeps at most 4 live, selected first then largest", () => {
    expect(MAX_LIVE_BROWSERS).toBe(4);
    const pages = [page("s", 0, 100), page("b1", 120, 300), page("b2", 440, 350), page("b3", 800, 380), page("b4", 1200, 390), page("b5", 1600, 200)];
    const r = planBrowsers({ pages, viewport, zoom: 1, gesturing: false, covers: [], selectedId: "s" });
    const live = r.filter((x) => x.rect).map((x) => x.id).sort();
    expect(live).toEqual(["b2", "b3", "b4", "s"]);
  });
});
