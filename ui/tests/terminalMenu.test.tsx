import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode, TerminalContent } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({ terminalStartFresh: vi.fn(async (_id: string, _confirmed: boolean) => {}) }));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { TerminalMenu } from "../src/nodes/TerminalMenu";
import { useCanvas } from "../src/state/canvas";

const conversation = { agent: "claude" as const, id: "7294d08d", recordedAt: "2026-10-24T13:05:00Z" };

function terminal(extra: Partial<TerminalContent> = {}): CanvasNode {
  return {
    id: "t1",
    frame: { x: 0, y: 0, width: 300, height: 200 },
    z: 1,
    createdAt: "",
    modifiedAt: "",
    content: { terminal: { name: "Api", saveHistory: true, ...extra } },
  };
}

async function openMenu(node: CanvasNode) {
  render(<TerminalMenu node={node} />);
  await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
}

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
});

describe("Start fresh", () => {
  it("is shown only when the terminal has a conversation", async () => {
    await openMenu(terminal());
    expect(screen.queryByRole("menuitem", { name: "Start fresh" })).not.toBeInTheDocument();
  });

  it("forgets the conversation and restarts", async () => {
    await openMenu(terminal({ conversation }));
    const item = screen.getByRole("menuitem", { name: "Start fresh" });
    expect(item).toHaveAttribute("title", "Forget this terminal's conversation and restart the agent in a new one.");
    await userEvent.click(item);
    expect(ipc.terminalStartFresh).toHaveBeenCalledWith("t1", false);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("asks for the restart confirmation when the terminal is busy", async () => {
    ipc.terminalStartFresh.mockRejectedValueOnce({ code: "TerminalBusy", message: "busy" });
    await openMenu(terminal({ conversation }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Start fresh" }));
    expect(await screen.findByText("Restart terminal?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Restart" }));
    expect(ipc.terminalStartFresh).toHaveBeenLastCalledWith("t1", true);
  });
});
