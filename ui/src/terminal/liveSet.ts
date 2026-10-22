import { intersects, type ScreenRect } from "../canvas/viewport";

/** Below this effective font size a terminal is shown as a preview (research R3). */
export const MIN_LEGIBLE_FONT_PX = 7;
/** WebKit allows about 16 WebGL contexts per page; stay well below. */
export const WEBGL_BUDGET = 12;
/** Previews are requested for terminals within this many screen pixels of the viewport. */
export const PREVIEW_MARGIN_PX = 400;

export type TerminalRect = { id: string; rect: ScreenRect };
export type Renderer = "webgl" | "dom";
export type LiveEntry = { id: string; renderer: Renderer };

type Input = {
  terminals: TerminalRect[];
  viewport: ScreenRect;
  zoom: number;
  baseFontSize: number;
  focusedId: string | null;
  /** Terminals whose WebGL context was lost; they use the DOM renderer from then on. */
  lost: Set<string>;
};

export function isLegible(zoom: number, baseFontSize: number) {
  return baseFontSize * zoom >= MIN_LEGIBLE_FONT_PX;
}

/**
 * Decides which terminals get a live xterm.js view (visible and legible), which of those use
 * WebGL (focused first, then the largest on screen, up to the budget), and which get previews.
 */
export function computeLiveSet({ terminals, viewport, zoom, baseFontSize, focusedId, lost }: Input) {
  const view = { left: 0, top: 0, width: viewport.width, height: viewport.height };
  const near = {
    left: -PREVIEW_MARGIN_PX,
    top: -PREVIEW_MARGIN_PX,
    width: viewport.width + 2 * PREVIEW_MARGIN_PX,
    height: viewport.height + 2 * PREVIEW_MARGIN_PX,
  };
  const legible = isLegible(zoom, baseFontSize);
  const visible = legible ? terminals.filter((t) => intersects(t.rect, view)) : [];
  const visibleIds = new Set(visible.map((t) => t.id));

  const area = (t: TerminalRect) => {
    const w = Math.min(t.rect.left + t.rect.width, view.width) - Math.max(t.rect.left, 0);
    const h = Math.min(t.rect.top + t.rect.height, view.height) - Math.max(t.rect.top, 0);
    return Math.max(0, w) * Math.max(0, h);
  };
  const ranked = [...visible].sort((a, b) => {
    if (a.id === focusedId) return -1;
    if (b.id === focusedId) return 1;
    return area(b) - area(a);
  });
  let budget = WEBGL_BUDGET;
  const renderer = new Map<string, Renderer>();
  for (const t of ranked) {
    if (budget > 0 && !lost.has(t.id)) {
      renderer.set(t.id, "webgl");
      budget--;
    } else {
      renderer.set(t.id, "dom");
    }
  }
  return {
    live: visible.map((t) => ({ id: t.id, renderer: renderer.get(t.id)! })),
    previews: terminals.filter((t) => !visibleIds.has(t.id) && intersects(t.rect, near)).map((t) => t.id),
  };
}
