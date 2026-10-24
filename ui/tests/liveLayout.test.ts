import { describe, expect, it } from "vitest";
import { MAX_LIVE_DEVICES, planLive } from "../src/canvas/liveLayout";

const viewport = { left: 0, top: 0, width: 2000, height: 1200 };
const item = (id: string, left: number, width = 300) => ({ id, rect: { left, top: 100, width, height: 600 } });

describe("device live set (007 FR-005)", () => {
  it("allows at most 2 live devices, selected first then largest", () => {
    expect(MAX_LIVE_DEVICES).toBe(2);
    const items = [item("s", 0, 100), item("a", 200, 300), item("b", 600, 350), item("c", 1000, 200)];
    const r = planLive({ items, viewport, zoom: 1, gesturing: false, covers: [], selectedId: "s", limit: MAX_LIVE_DEVICES });
    expect(r.filter((x) => x.rect).map((x) => x.id).sort()).toEqual(["b", "s"]);
  });

  it("uses the same visibility rules as browsers", () => {
    const base = { items: [item("a", 10)], viewport, zoom: 1, gesturing: false, covers: [], selectedId: null, limit: 2 };
    expect(planLive(base)[0].rect).not.toBeNull();
    expect(planLive({ ...base, gesturing: true })[0].rect).toBeNull();
    expect(planLive({ ...base, zoom: 0.3 })[0].rect).toBeNull();
    expect(planLive({ ...base, covers: [{ left: 50, top: 200, width: 20, height: 20 }] })[0].rect).toBeNull();
  });
});
