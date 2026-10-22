import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  noteRead: vi.fn(),
  noteWrite: vi.fn(async () => {}),
  nodeDelete: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { NoteNode } from "../src/nodes/NoteNode";
import { useCanvas } from "../src/state/canvas";

const node: CanvasNode = {
  id: "n1",
  frame: { x: 0, y: 0, width: 300, height: 200 },
  z: 1,
  createdAt: "",
  modifiedAt: "",
  content: { note: {} },
};

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
  useCanvas.setState({ workspaceId: "w1", nodes: [node] });
  ipc.noteRead.mockResolvedValue({ text: "# Title\n\n- [x] done\n\n| a | b |\n|---|---|\n| 1 | 2 |", sync: "clean" });
});

describe("note node", () => {
  it("renders Markdown with GFM", async () => {
    render(<NoteNode node={node} />);
    expect(await screen.findByRole("heading", { name: "Title" })).toBeInTheDocument();
    expect(screen.getByRole("checkbox")).toBeChecked();
    expect(screen.getByRole("table")).toBeInTheDocument();
  });

  it("double-click edits in a textarea and blur saves and renders", async () => {
    render(<NoteNode node={node} />);
    await screen.findByRole("heading", { name: "Title" });
    await userEvent.dblClick(screen.getByTestId("note-view"));
    const box = screen.getByRole("textbox", { name: "Note text" });
    await userEvent.clear(box);
    await userEvent.type(box, "## Next");
    act(() => box.blur());
    await waitFor(() => expect(ipc.noteWrite).toHaveBeenLastCalledWith("n1", "## Next", true));
    expect(await screen.findByRole("heading", { name: "Next" })).toBeInTheDocument();
  });

  it("shows a warning on conflict and on missing file", async () => {
    render(<NoteNode node={node} />);
    await screen.findByRole("heading", { name: "Title" });
    act(() => useCanvas.getState().setNoteConflict("n1", "/data/notes/n1 (conflict).md"));
    expect(await screen.findByRole("alert")).toHaveTextContent("n1 (conflict).md");
    act(() => useCanvas.getState().setNoteText("n1", "kept", "fileMissing"));
    expect(await screen.findByRole("alert")).toHaveTextContent(/deleted outside Sonata/);
  });
});
