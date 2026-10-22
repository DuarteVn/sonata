import type { PointerEvent as ReactPointerEvent } from "react";

/** Starts a pointer drag; `onMove` gets the total screen-pixel delta since the start. */
export function startDrag(
  e: ReactPointerEvent,
  onMove: (dx: number, dy: number) => void,
  onEnd: (dx: number, dy: number) => void,
) {
  const el = e.currentTarget as HTMLElement;
  const startX = e.clientX;
  const startY = e.clientY;
  el.setPointerCapture(e.pointerId);
  const move = (ev: PointerEvent) => onMove(ev.clientX - startX, ev.clientY - startY);
  const up = (ev: PointerEvent) => {
    el.removeEventListener("pointermove", move);
    el.removeEventListener("pointerup", up);
    el.removeEventListener("pointercancel", up);
    onEnd(ev.clientX - startX, ev.clientY - startY);
  };
  el.addEventListener("pointermove", move);
  el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", up);
}
