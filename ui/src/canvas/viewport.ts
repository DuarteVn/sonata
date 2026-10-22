import type { Camera, Frame } from "../ipc/types";

// Camera convention: (x, y) is the canvas point at the screen's top-left corner.
// screen = (canvas - camera) * zoom

export const ZOOM_MIN = 0.1;
export const ZOOM_MAX = 4;

export type Point = { x: number; y: number };
export type ScreenRect = { left: number; top: number; width: number; height: number };

export const clampZoom = (zoom: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom));

export function canvasToScreen(cam: Camera, p: Point): Point {
  return { x: (p.x - cam.x) * cam.zoom, y: (p.y - cam.y) * cam.zoom };
}

export function screenToCanvas(cam: Camera, p: Point): Point {
  return { x: p.x / cam.zoom + cam.x, y: p.y / cam.zoom + cam.y };
}

export function frameToScreen(cam: Camera, f: Frame): ScreenRect {
  const { x, y } = canvasToScreen(cam, f);
  return { left: x, top: y, width: f.width * cam.zoom, height: f.height * cam.zoom };
}

/** Multiplies zoom by `factor` (clamped) while keeping the canvas point under `pointer` fixed. */
export function zoomAt(cam: Camera, pointer: Point, factor: number): Camera {
  const zoom = clampZoom(cam.zoom * factor);
  const anchor = screenToCanvas(cam, pointer);
  return { x: anchor.x - pointer.x / zoom, y: anchor.y - pointer.y / zoom, zoom };
}

/** Moves the view by screen pixels (positive dx moves content right). */
export function pan(cam: Camera, dx: number, dy: number): Camera {
  return { x: cam.x - dx / cam.zoom, y: cam.y - dy / cam.zoom, zoom: cam.zoom };
}

export function intersects(a: ScreenRect, b: ScreenRect): boolean {
  return a.left < b.left + b.width && b.left < a.left + a.width && a.top < b.top + b.height && b.top < a.top + a.height;
}
