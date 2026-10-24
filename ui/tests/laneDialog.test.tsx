import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { LanePreview } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  lanePreview: vi.fn(),
  laneLand: vi.fn(),
  laneRemove: vi.fn(),
  laneConfirmAnswer: vi.fn(),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { LaneDialog } from "../src/dialogs/LaneDialog";

const preview = (over: Partial<LanePreview> = {}): LanePreview => ({
  branch: "lane/api",
  baseBranch: "main",
  commits: 2,
  changedFiles: [],
  changedTotal: 0,
  unmergedCommits: 2,
  blockers: [],
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  ipc.lanePreview.mockResolvedValue(preview());
  ipc.laneLand.mockResolvedValue(2);
  ipc.laneRemove.mockResolvedValue(undefined);
  ipc.laneConfirmAnswer.mockResolvedValue(undefined);
});

describe("Land", () => {
  it("shows what will be merged and lands on confirm", async () => {
    const onClose = vi.fn();
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={onClose} />);
    expect(await screen.findByText(/lane\/api/)).toBeInTheDocument();
    expect(screen.getByText(/into main/)).toBeInTheDocument();
    expect(screen.getByText(/2 commits/)).toBeInTheDocument();
    expect(ipc.lanePreview).toHaveBeenCalledWith("t1", "land");
    await userEvent.click(screen.getByRole("button", { name: "Land" }));
    await waitFor(() => expect(ipc.laneLand).toHaveBeenCalledWith("t1", true));
    expect(onClose).toHaveBeenCalled();
  });

  it("lists blockers and disables Land", async () => {
    ipc.lanePreview.mockResolvedValue(preview({ blockers: ["The lane has 3 uncommitted files. Commit them first."] }));
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={() => {}} />);
    expect(await screen.findByText(/3 uncommitted files/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Land" })).toBeDisabled();
  });

  it("keeps the dialog open with the conflict list", async () => {
    ipc.laneLand.mockRejectedValue({ code: "LaneConflict", message: "Land cancelled: conflicts in a.txt. Nothing changed." });
    const onClose = vi.fn();
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={onClose} />);
    await userEvent.click(await screen.findByRole("button", { name: "Land" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("conflicts in a.txt");
    expect(onClose).not.toHaveBeenCalled();
  });

  it("cancel changes nothing", async () => {
    const onClose = vi.fn();
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={onClose} />);
    await userEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(onClose).toHaveBeenCalled();
    expect(ipc.laneLand).not.toHaveBeenCalled();
  });

  it("says there is nothing to land", async () => {
    ipc.lanePreview.mockResolvedValue(preview({ commits: 0 }));
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={() => {}} />);
    expect(await screen.findByText(/Nothing to land/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Land" })).toBeDisabled();
  });
});

describe("Terminal menu lane items", () => {
  it("offers Land only for an active lane and opens the dialog", async () => {
    const { TerminalMenu } = await import("../src/nodes/TerminalMenu");
    const { useCanvas } = await import("../src/state/canvas");
    useCanvas.setState({ laneDialog: null });
    const lane = { branch: "lane/api", baseBranch: "main", mainCheckout: "/r", checkoutPath: "/d/api", createdAt: "" };
    const node = {
      id: "t1", frame: { x: 0, y: 0, width: 300, height: 200 }, z: 1, createdAt: "", modifiedAt: "",
      content: { terminal: { name: "Api", saveHistory: true, lane } },
    };
    const { unmount } = render(<TerminalMenu node={node} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    await userEvent.click(screen.getByRole("menuitem", { name: /Land/ }));
    expect(useCanvas.getState().laneDialog).toEqual({ nodeId: "t1", action: "land" });
    unmount();

    const landed = { ...node, content: { terminal: { name: "Api", saveHistory: true, lane: { ...lane, landedAt: "x" } } } };
    render(<TerminalMenu node={landed} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    expect(screen.queryByRole("menuitem", { name: /Land/ })).not.toBeInTheDocument();
  });
});
