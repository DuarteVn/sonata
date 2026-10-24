import { MIN_LIVE_ZOOM, planLive } from "./liveLayout";
import type { ScreenRect } from "./viewport";

/** At most this many pages are live at once (004 FR-007). */
export const MAX_LIVE_BROWSERS = 4;
export { MIN_LIVE_ZOOM };

type Input = {
  pages: { id: string; rect: ScreenRect }[];
  viewport: ScreenRect;
  zoom: number;
  gesturing: boolean;
  covers: ScreenRect[];
  selectedId: string | null;
};

/** Which browser pages are live and where; `rect: null` means "show the card". */
export function planBrowsers({ pages, ...rest }: Input) {
  return planLive({ items: pages, ...rest, limit: MAX_LIVE_BROWSERS });
}
