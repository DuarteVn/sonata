// Geometry and input mapping for the live device screen (feature 007, FR-006). Pure functions:
// the node passes on-screen rects (already zoomed), so the same math works at any zoom.

export type Size = { width: number; height: number };
export type Rect = { left: number; top: number; width: number; height: number };
export type Point = { x: number; y: number };

/** Most pixels per streamed frame (research R2). */
export const MAX_STREAM_PIXELS = 1_200_000;

export type DeviceInput =
  | { touch: { x: number; y: number; down: boolean } }
  | { wheel: { x: number; y: number; dx: number; dy: number } }
  | { text: string }
  | { key: string };

/** The device screen inside the node body, letterboxed to keep the device's aspect ratio. */
export function fitScreen(body: Size, device: Size): Rect {
  const scale = Math.min(body.width / device.width, body.height / device.height);
  const width = Math.round(device.width * scale);
  const height = Math.round(device.height * scale);
  return { left: Math.round((body.width - width) / 2), top: Math.round((body.height - height) / 2), width, height };
}

/** A point on screen to device pixels; `null` outside the screen element's rect. */
export function toDevice(client: Point, rect: Rect, device: Size): Point | null {
  const fx = (client.x - rect.left) / rect.width;
  const fy = (client.y - rect.top) / rect.height;
  if (fx < 0 || fy < 0 || fx > 1 || fy > 1) return null;
  return {
    x: Math.min(device.width - 1, Math.round(fx * device.width)),
    y: Math.min(device.height - 1, Math.round(fy * device.height)),
  };
}

/** Frame size to ask for: physical pixels of the screen area, capped by the device and about 1.2 MP. */
export function streamSize(screen: Size, pixelRatio: number, device: Size): Size {
  let scale = Math.min((screen.width * pixelRatio) / device.width, (screen.height * pixelRatio) / device.height, 1);
  const pixels = device.width * device.height * scale * scale;
  if (pixels > MAX_STREAM_PIXELS) scale *= Math.sqrt(MAX_STREAM_PIXELS / pixels);
  return { width: Math.max(1, Math.round(device.width * scale)), height: Math.max(1, Math.round(device.height * scale)) };
}

/** One finger that follows the mouse while the button is held (taps, long presses, swipes). */
export class TouchTracker {
  private last: Point | null = null;

  down(p: Point): DeviceInput {
    this.last = p;
    return { touch: { ...p, down: true } };
  }

  move(p: Point | null): DeviceInput | null {
    if (!this.last || !p || (p.x === this.last.x && p.y === this.last.y)) return null;
    this.last = p;
    return { touch: { ...p, down: true } };
  }

  /** `p` is null when the pointer left the screen; the finger lifts where it last was. */
  up(p: Point | null): DeviceInput | null {
    if (!this.last) return null;
    const at = p ?? this.last;
    this.last = null;
    return { touch: { ...at, down: false } };
  }

  get pressed() {
    return this.last !== null;
  }
}

/** Browser wheel deltas (about 100 per notch, down positive) to emulator units (120 per click, up positive). */
export function wheelToDevice(p: Point, deltaX: number, deltaY: number): { wheel: { x: number; y: number; dx: number; dy: number } } {
  const units = (d: number) => Math.max(-1200, Math.min(1200, -Math.round(d * 1.2))) || 0;
  return { wheel: { ...p, dx: units(deltaX), dy: units(deltaY) } };
}

const NAMED_KEYS = new Set(["Enter", "Backspace", "Tab", "Escape", "Delete", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End"]);

/** Printable characters become text; editing and arrow keys go by name; shortcuts are ignored. */
export function keyToInput(e: { key: string; metaKey: boolean; ctrlKey: boolean }): DeviceInput | null {
  if (e.metaKey || e.ctrlKey) return null;
  if (e.key.length === 1) return { text: e.key };
  if (NAMED_KEYS.has(e.key)) return { key: e.key };
  return null;
}
