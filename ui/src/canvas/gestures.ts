// Gesture routing rules from FR-008a, kept pure so they are easy to test.

export type Target = { kind: "terminal"; id: string } | { kind: "note"; id: string } | { kind: "empty" };

export type WheelInput = {
  /** macOS reports trackpad pinch as wheel events with ctrlKey set. */
  ctrlKey: boolean;
  deltaX: number;
  deltaY: number;
  target: Target;
  focusedId: string | null;
};

export type WheelAction =
  | { type: "scrollTerminal"; id: string }
  | { type: "pan"; dx: number; dy: number }
  | { type: "zoom"; factor: number };

export function routeWheel(input: WheelInput): WheelAction {
  if (input.ctrlKey) return { type: "zoom", factor: Math.exp(-input.deltaY * 0.01) };
  if (input.target.kind === "terminal" && input.target.id === input.focusedId) {
    return { type: "scrollTerminal", id: input.target.id };
  }
  return { type: "pan", dx: -input.deltaX, dy: -input.deltaY };
}

export type PointerInput = {
  button: number;
  spaceHeld: boolean;
  /** True when a terminal or a text field has keyboard focus. */
  keyboardFocus: boolean;
  target: Target;
};

export type PointerAction = "pan" | "clearFocus" | "none";

export function routePointerDown(input: PointerInput): PointerAction {
  if (input.button === 1) return "pan";
  if (input.button === 0 && input.spaceHeld && !input.keyboardFocus) return "pan";
  if (input.button === 0 && input.target.kind === "empty") return "clearFocus";
  return "none";
}

/** Reads the canvas target under a DOM element (nodes carry data-node-id and data-node-kind). */
export function targetOf(el: Element | null): Target {
  const node = el?.closest<HTMLElement>("[data-node-id]");
  if (!node) return { kind: "empty" };
  const id = node.dataset.nodeId!;
  return node.dataset.nodeKind === "note" ? { kind: "note", id } : { kind: "terminal", id };
}
