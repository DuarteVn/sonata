import { intersects, type ScreenRect } from "./viewport";

/** At most this many device screens stream at once (007 FR-005). */
export const MAX_LIVE_DEVICES = 2;
/** Below this zoom native pages and device screens show cards. */
export const MIN_LIVE_ZOOM = 0.55;

type Input = {
  /** Areas in canvas-container coordinates. */
  items: { id: string; rect: ScreenRect }[];
  viewport: ScreenRect;
  zoom: number;
  gesturing: boolean;
  /** Interface elements (dialogs, menus, toolbars) in the same coordinates. */
  covers: ScreenRect[];
  selectedId: string | null;
  limit: number;
};

/**
 * Which items are live and where; `rect: null` means "show the card". Live only when fully
 * visible at a legible zoom, uncovered, with no gesture running; selected first, then largest.
 */
export function planLive({ items, viewport, zoom, gesturing, covers, selectedId, limit }: Input) {
  const view = { left: 0, top: 0, width: viewport.width, height: viewport.height };
  const inside = (r: ScreenRect) =>
    r.left >= 0 && r.top >= 0 && r.left + r.width <= view.width && r.top + r.height <= view.height;
  const eligible =
    gesturing || zoom < MIN_LIVE_ZOOM
      ? []
      : items.filter((p) => intersects(p.rect, view) && inside(p.rect) && !covers.some((c) => intersects(c, p.rect)));
  const ranked = [...eligible].sort((a, b) => {
    if (a.id === selectedId) return -1;
    if (b.id === selectedId) return 1;
    return b.rect.width * b.rect.height - a.rect.width * a.rect.height;
  });
  const live = new Set(ranked.slice(0, limit).map((p) => p.id));
  return items.map((p) => ({ id: p.id, rect: live.has(p.id) ? p.rect : null }));
}
