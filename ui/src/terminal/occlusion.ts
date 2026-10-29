import { intersects, type ScreenRect } from "../canvas/viewport";

/** `a` minus `b` as up to four disjoint rectangles. */
function subtract(a: ScreenRect, b: ScreenRect): ScreenRect[] {
  if (!intersects(a, b)) return [a];
  const aRight = a.left + a.width;
  const aBottom = a.top + a.height;
  const top = Math.max(a.top, b.top);
  const bottom = Math.min(aBottom, b.top + b.height);
  const out: ScreenRect[] = [];
  if (b.top > a.top) out.push({ left: a.left, top: a.top, width: a.width, height: b.top - a.top });
  if (b.top + b.height < aBottom) out.push({ left: a.left, top: bottom, width: a.width, height: aBottom - bottom });
  if (b.left > a.left) out.push({ left: a.left, top, width: b.left - a.left, height: bottom - top });
  if (b.left + b.width < aRight) out.push({ left: b.left + b.width, top, width: aRight - b.left - b.width, height: bottom - top });
  return out;
}

/**
 * CSS `clip-path` that hides the parts of a live terminal covered by nodes stacked above it.
 * Live views sit in their own layer above every node frame, so without it a lower terminal
 * draws over the frame of a higher one. `undefined` means nothing covers the view.
 */
export function occlusionClip(body: ScreenRect, covers: ScreenRect[]): string | undefined {
  const hits = covers.filter((c) => intersects(c, body));
  if (hits.length === 0) return undefined;
  let visible = [body];
  for (const c of hits) visible = visible.flatMap((r) => subtract(r, c));
  if (visible.length === 0) return "inset(50%)";
  const d = visible
    .map((r) => {
      const x = r.left - body.left;
      const y = r.top - body.top;
      return `M${x},${y}h${r.width}v${r.height}h${-r.width}z`;
    })
    .join("");
  return `path("${d}")`;
}
