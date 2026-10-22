import { beforeEach, describe, expect, it } from "vitest";
import { useCanvas } from "../src/state/canvas";

beforeEach(() => useCanvas.setState(useCanvas.getInitialState(), true));

const state = (busy: boolean) => ({ nodeId: "t1", run: "running" as const, busy });

describe("attention badge (FR-013)", () => {
  it("is set when a busy terminal becomes idle while not focused, cleared on focus", () => {
    const s = useCanvas.getState();
    s.setTerminalState(state(true));
    s.setTerminalState(state(false));
    expect(useCanvas.getState().attention.has("t1")).toBe(true);
    useCanvas.getState().setFocus("t1");
    expect(useCanvas.getState().attention.has("t1")).toBe(false);
  });

  it("is not set for the focused terminal", () => {
    useCanvas.getState().setFocus("t1");
    useCanvas.getState().setTerminalState(state(true));
    useCanvas.getState().setTerminalState(state(false));
    expect(useCanvas.getState().attention.has("t1")).toBe(false);
  });
});
