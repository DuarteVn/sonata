import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  browserNavigate: vi.fn(async (_id: string, input: string) => (input.startsWith("localhost") ? `http://${input}` : `https://${input}`)),
  browserAction: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string) => `asset://${p}` }));

import { BrowserNode } from "../src/nodes/BrowserNode";
import { useCanvas } from "../src/state/canvas";

const node: CanvasNode = {
  id: "b1",
  frame: { x: 0, y: 0, width: 1024, height: 720 },
  z: 1,
  createdAt: "",
  modifiedAt: "",
  content: { browser: { url: "about:blank" } },
};

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
  useCanvas.setState({ nodes: [node], workspaceId: "w" });
});

describe("browser node", () => {
  it("navigates from the address field", async () => {
    render(<BrowserNode node={node} />);
    const address = screen.getByRole("textbox", { name: "Address" });
    await userEvent.type(address, "localhost:3000{Enter}");
    expect(ipc.browserNavigate).toHaveBeenCalledWith("b1", "localhost:3000");
  });

  it("chrome buttons call browser actions", async () => {
    render(<BrowserNode node={node} />);
    for (const [label, action] of [["Back", "back"], ["Forward", "forward"], ["Reload", "reload"], ["Open in default browser", "external"], ["Inspect", "inspect"]]) {
      await userEvent.click(screen.getByRole("button", { name: label }));
      expect(ipc.browserAction).toHaveBeenLastCalledWith("b1", action);
    }
  });

  it("shows a card with title, URL, and image when not live", () => {
    useCanvas.getState().setBrowserState({ nodeId: "b1", url: "https://example.com/", title: "Example", loading: false, image: "/d/b1.png", imageVersion: 2, live: false });
    render(<BrowserNode node={node} />);
    expect(screen.getByText("Example")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "Last view of Example" })).toHaveAttribute("src", "asset:///d/b1.png?v=2");
  });
});
