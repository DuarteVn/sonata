import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { act } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  exchangeLog: vi.fn(async () => [
    { at: "2026-10-22T10:00:00Z", direction: "in", other: "lead", firstLine: "Run the tests", outcome: "answered", resultChars: 120 },
  ]),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { LinkLayer } from "../src/canvas/LinkLayer";
import { MessageLog } from "../src/nodes/MessageLog";
import { TerminalNode } from "../src/nodes/TerminalNode";
import { useCanvas } from "../src/state/canvas";

const term = (id: string, name: string, x: number): CanvasNode & { content: { terminal: { name: string; saveHistory: boolean } } } => ({
  id,
  frame: { x, y: 0, width: 300, height: 200 },
  z: 1,
  createdAt: "",
  modifiedAt: "",
  content: { terminal: { name, saveHistory: true } },
});

beforeEach(() => {
  useCanvas.setState(useCanvas.getInitialState(), true);
  useCanvas.setState({
    nodes: [term("lead", "Lead", 0), term("worker", "Worker", 500)],
    links: [{ id: "l1", a: "lead", b: "worker", createdAt: "" }],
  });
});

describe("messaging visibility (FR-027, FR-028)", () => {
  it("highlights the link and shows who is asking while an ask waits", () => {
    const { container } = render(
      <>
        <LinkLayer drag={null} />
        <TerminalNode node={term("worker", "Worker", 500)} />
      </>,
    );
    expect(container.querySelector(".link.busy")).toBeNull();
    act(() => useCanvas.getState().setAsk({ askId: "k", from: "lead", to: "worker", state: "waiting" }));
    expect(container.querySelector(".link.busy")).not.toBeNull();
    expect(screen.getByText("asked by Lead")).toBeInTheDocument();
    act(() => useCanvas.getState().setAsk({ askId: "k", from: "lead", to: "worker", state: "answered" }));
    expect(container.querySelector(".link.busy")).toBeNull();
    expect(screen.queryByText("asked by Lead")).toBeNull();
  });

  it("lists recent exchanges", async () => {
    render(<MessageLog nodeId="worker" onClose={() => {}} />);
    expect(await screen.findByText("Run the tests")).toBeInTheDocument();
    expect(screen.getByText(/from Lead/)).toBeInTheDocument();
    expect(screen.getByText(/answered/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Close" }));
  });
});
