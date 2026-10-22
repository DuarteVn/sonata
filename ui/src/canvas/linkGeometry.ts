import type { Frame } from "../ipc/types";

export type Side = "left" | "right" | "top" | "bottom" | "center";
export type Anchor = { x: number; y: number; side: Side };

const center = (f: Frame) => ({ x: f.x + f.width / 2, y: f.y + f.height / 2 });

/** Attachment points on the facing edges of two frames (centers when they overlap). */
export function linkAnchors(a: Frame, b: Frame): [Anchor, Anchor] {
  const ca = center(a);
  const cb = center(b);
  const gapX = Math.max(b.x - (a.x + a.width), a.x - (b.x + b.width));
  const gapY = Math.max(b.y - (a.y + a.height), a.y - (b.y + b.height));
  if (gapX <= 0 && gapY <= 0) {
    return [
      { ...ca, side: "center" },
      { ...cb, side: "center" },
    ];
  }
  if (gapX >= gapY) {
    return cb.x >= ca.x
      ? [
          { x: a.x + a.width, y: ca.y, side: "right" },
          { x: b.x, y: cb.y, side: "left" },
        ]
      : [
          { x: a.x, y: ca.y, side: "left" },
          { x: b.x + b.width, y: cb.y, side: "right" },
        ];
  }
  return cb.y >= ca.y
    ? [
        { x: ca.x, y: a.y + a.height, side: "bottom" },
        { x: cb.x, y: b.y, side: "top" },
      ]
    : [
        { x: ca.x, y: a.y, side: "top" },
        { x: cb.x, y: b.y + b.height, side: "bottom" },
      ];
}

function control(p: Anchor, dist: number) {
  switch (p.side) {
    case "left":
      return { x: p.x - dist, y: p.y };
    case "right":
      return { x: p.x + dist, y: p.y };
    case "top":
      return { x: p.x, y: p.y - dist };
    case "bottom":
      return { x: p.x, y: p.y + dist };
    default:
      return { x: p.x, y: p.y };
  }
}

/** SVG cubic Bézier from `p` to `q`, leaving each anchor perpendicular to its edge. */
export function linkPath(p: Anchor, q: Anchor): string {
  const dist = Math.max(40, Math.hypot(q.x - p.x, q.y - p.y) * 0.4);
  const c1 = control(p, dist);
  const c2 = control(q, dist);
  return `M ${p.x} ${p.y} C ${c1.x} ${c1.y}, ${c2.x} ${c2.y}, ${q.x} ${q.y}`;
}

/** Anchor for a free end (the pointer while dragging a new link). */
export function pointAnchor(x: number, y: number): Anchor {
  return { x, y, side: "center" };
}
