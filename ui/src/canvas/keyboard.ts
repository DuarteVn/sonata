// Canvas keyboard rules (FR-002, FR-004, FR-005). Keys apply only when neither a terminal nor a
// text field has keyboard focus, so typing never triggers canvas actions.

export type KeyInput = {
  key: string;
  textFocus: boolean;
  terminalFocus: boolean;
  hasSelection: boolean;
  linking: boolean;
  meta?: boolean;
};

export type KeyAction = "tool:select" | "tool:terminal" | "tool:note" | "tool:browser" | "delete" | "clear" | "cancelLink";

export function routeKey(input: KeyInput): KeyAction | null {
  if (input.textFocus || input.terminalFocus || input.meta) return null;
  switch (input.key) {
    case "v":
    case "V":
      return "tool:select";
    case "t":
    case "T":
      return "tool:terminal";
    case "n":
    case "N":
      return "tool:note";
    case "b":
    case "B":
      return "tool:browser";
    case "Delete":
    case "Backspace":
      return input.hasSelection ? "delete" : null;
    case "Escape":
      return input.linking ? "cancelLink" : "clear";
    default:
      return null;
  }
}
