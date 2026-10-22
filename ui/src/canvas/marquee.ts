import type { CanvasNode } from "../ipc/types";
import { intersects, type ScreenRect } from "./viewport";

/** A rectangle from two corners in any order. */
export function rectFromPoints(a: { x: number; y: number }, b: { x: number; y: number }): ScreenRect {
  return { left: Math.min(a.x, b.x), top: Math.min(a.y, b.y), width: Math.abs(a.x - b.x), height: Math.abs(a.y - b.y) };
}

/** Ids of the nodes that a selection rectangle (canvas coordinates) touches. */
export function nodesInRect(nodes: CanvasNode[], rect: ScreenRect): string[] {
  return nodes
    .filter((n) => intersects(rect, { left: n.frame.x, top: n.frame.y, width: n.frame.width, height: n.frame.height }))
    .map((n) => n.id);
}
