import { describe, expect, it } from "vitest";
import { fitScreen, keyToInput, streamSize, toDevice, TouchTracker, wheelToDevice } from "../src/nodes/devicePointer";

const device = { width: 1080, height: 2400 };

describe("device screen geometry (FR-006, AS5)", () => {
  it("letterboxes the device screen inside the node body, keeping the aspect ratio", () => {
    // Tall body: full width, centered vertically.
    expect(fitScreen({ width: 360, height: 900 }, device)).toEqual({ left: 0, top: 50, width: 360, height: 800 });
    // Wide body: full height, centered horizontally.
    expect(fitScreen({ width: 600, height: 800 }, device)).toEqual({ left: 120, top: 0, width: 360, height: 800 });
  });

  it("maps a click to device pixels at any node size and zoom", () => {
    // The screen element's on-screen rect already includes zoom and position.
    for (const rect of [
      { left: 100, top: 50, width: 360, height: 800 },
      { left: 0, top: 0, width: 180, height: 400 },
      { left: -40, top: 20, width: 720, height: 1600 },
    ]) {
      const center = toDevice({ x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }, rect, device);
      expect(center).toEqual({ x: 540, y: 1200 });
      const corner = toDevice({ x: rect.left + rect.width * 0.25, y: rect.top + rect.height * 0.75 }, rect, device);
      expect(corner).toEqual({ x: 270, y: 1800 });
    }
    expect(toDevice({ x: 10, y: 10 }, { left: 100, top: 100, width: 360, height: 800 }, device)).toBeNull();
  });

  it("streams at the screen's physical size, never above the device size or about 1.2 MP", () => {
    expect(streamSize({ width: 360, height: 800 }, 2, device)).toEqual({ width: 720, height: 1600 });
    expect(streamSize({ width: 900, height: 2000 }, 2, device)).toEqual({ width: 735, height: 1633 });
    expect(streamSize({ width: 100, height: 222 }, 1, device)).toEqual({ width: 100, height: 222 });
  });
});

describe("device input (FR-006)", () => {
  it("turns pointer down/move/up into one finger that follows the mouse", () => {
    const t = new TouchTracker();
    expect(t.down({ x: 10, y: 20 })).toEqual({ touch: { x: 10, y: 20, down: true } });
    expect(t.move({ x: 12, y: 40 })).toEqual({ touch: { x: 12, y: 40, down: true } });
    expect(t.move({ x: 12, y: 40 })).toBeNull();
    expect(t.up({ x: 12, y: 60 })).toEqual({ touch: { x: 12, y: 60, down: false } });
    // Moves without a press are not touches.
    expect(t.move({ x: 1, y: 1 })).toBeNull();
    expect(t.up(null)).toBeNull();
  });

  it("releases at the last position when the pointer leaves the screen", () => {
    const t = new TouchTracker();
    t.down({ x: 10, y: 20 });
    t.move({ x: 30, y: 40 });
    expect(t.up(null)).toEqual({ touch: { x: 30, y: 40, down: false } });
  });

  it("maps wheel deltas to emulator wheel units (120 per click, up is positive)", () => {
    expect(wheelToDevice({ x: 5, y: 6 }, 0, 100)).toEqual({ wheel: { x: 5, y: 6, dx: 0, dy: -120 } });
    expect(wheelToDevice({ x: 5, y: 6 }, 0, -50)).toEqual({ wheel: { x: 5, y: 6, dx: 0, dy: 60 } });
    expect(wheelToDevice({ x: 5, y: 6 }, 0, 100000).wheel.dy).toBe(-1200);
  });

  it("sends printable keys as text and special keys by name", () => {
    expect(keyToInput({ key: "a", metaKey: false, ctrlKey: false })).toEqual({ text: "a" });
    expect(keyToInput({ key: " ", metaKey: false, ctrlKey: false })).toEqual({ text: " " });
    expect(keyToInput({ key: "Enter", metaKey: false, ctrlKey: false })).toEqual({ key: "Enter" });
    expect(keyToInput({ key: "Backspace", metaKey: false, ctrlKey: false })).toEqual({ key: "Backspace" });
    expect(keyToInput({ key: "ArrowLeft", metaKey: false, ctrlKey: false })).toEqual({ key: "ArrowLeft" });
    expect(keyToInput({ key: "Shift", metaKey: false, ctrlKey: false })).toBeNull();
    expect(keyToInput({ key: "c", metaKey: true, ctrlKey: false })).toBeNull();
  });
});
