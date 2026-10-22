import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  nodeDelete: vi.fn(async (_ws: string, id: string, confirmed: boolean) => {
    if (id === "busy" && !confirmed) throw { code: "TerminalBusy", message: "busy" };
  }),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { useCanvas } from "../src/state/canvas";

const node = (id: string, kind: "note" | "terminal") =>
  ({
    id,
    frame: { x: 0, y: 0, width: 100, height: 100 },
    z: 0,
    content: kind === "note" ? { note: {} } : { terminal: { name: id } },
  }) as unknown as CanvasNode;

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState({
    workspaceId: "w",
    nodes: [node("a", "note"), node("b", "terminal"), node("busy", "terminal")],
    links: [],
    selection: null,
    group: [],
    confirmCloseMany: null,
  });
});

describe("group selection", () => {
  it("one id is a plain selection; several form a group", () => {
    const s = useCanvas.getState();
    s.setGroup(["a"]);
    expect(useCanvas.getState().selection).toEqual({ kind: "node", id: "a" });
    s.setGroup(["a", "b"]);
    expect(useCanvas.getState()).toMatchObject({ selection: null, group: ["a", "b"] });
  });

  it("Shift+press toggles membership; a plain press on a member keeps the group", () => {
    const s = useCanvas.getState();
    s.select({ kind: "node", id: "a" });
    s.pressNode("b", "terminal", true);
    expect(useCanvas.getState().group).toEqual(["a", "b"]);
    s.pressNode("a", "note");
    expect(useCanvas.getState().group).toEqual(["a", "b"]);
    s.pressNode("b", "terminal", true);
    expect(useCanvas.getState().selection).toEqual({ kind: "node", id: "a" });
  });

  it("deletes the whole group and asks once for terminals still running a program", async () => {
    useCanvas.getState().setGroup(["a", "b", "busy"]);
    await useCanvas.getState().deleteSelection();
    expect(ipc.nodeDelete).toHaveBeenCalledWith("w", "a", true);
    expect(ipc.nodeDelete).toHaveBeenCalledWith("w", "b", false);
    expect(useCanvas.getState().nodes.map((n) => n.id)).toEqual(["busy"]);
    expect(useCanvas.getState().confirmCloseMany).toEqual(["busy"]);
  });
});
