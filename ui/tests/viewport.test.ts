import { describe, expect, it } from "vitest";
import {
  ZOOM_MAX,
  ZOOM_MIN,
  canvasToScreen,
  frameToScreen,
  pan,
  screenToCanvas,
  zoomAt,
} from "../src/canvas/viewport";

const cam = { x: 100, y: 50, zoom: 2 };

describe("viewport math", () => {
  it("converts between canvas and screen", () => {
    expect(canvasToScreen(cam, { x: 110, y: 60 })).toEqual({ x: 20, y: 20 });
    expect(screenToCanvas(cam, { x: 20, y: 20 })).toEqual({ x: 110, y: 60 });
  });

  it("maps a frame to a screen rectangle", () => {
    expect(frameToScreen(cam, { x: 100, y: 50, width: 10, height: 5 })).toEqual({
      left: 0,
      top: 0,
      width: 20,
      height: 10,
    });
  });

  it("zooms around the pointer, keeping that canvas point fixed", () => {
    const pointer = { x: 300, y: 200 };
    const before = screenToCanvas(cam, pointer);
    const next = zoomAt(cam, pointer, 1.5);
    expect(next.zoom).toBeCloseTo(3);
    const after = screenToCanvas(next, pointer);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
  });

  it("clamps zoom to [0.1, 4]", () => {
    expect(zoomAt(cam, { x: 0, y: 0 }, 100).zoom).toBe(ZOOM_MAX);
    expect(zoomAt(cam, { x: 0, y: 0 }, 0.0001).zoom).toBe(ZOOM_MIN);
    expect(ZOOM_MIN).toBe(0.1);
    expect(ZOOM_MAX).toBe(4);
  });

  it("pans by screen pixels", () => {
    expect(pan(cam, 20, -10)).toEqual({ x: 90, y: 55, zoom: 2 });
  });
});
