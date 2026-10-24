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
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    await userEvent.click(screen.getByRole("menuitem", { name: "Remove lane…" }));
    expect(useCanvas.getState().laneDialog).toEqual({ nodeId: "t1", action: "remove" });
    unmount();

    const landed = { ...node, content: { terminal: { name: "Api", saveHistory: true, lane: { ...lane, landedAt: "x" } } } };
    render(<TerminalMenu node={landed} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    expect(screen.queryByRole("menuitem", { name: /Land/ })).not.toBeInTheDocument();
  });
});

describe("Remove lane", () => {
  it("removes a clean lane and keeps the branch by default", async () => {
    const onClose = vi.fn();
    render(<LaneDialog state={{ nodeId: "t1", action: "remove" }} onClose={onClose} />);
    expect(await screen.findByText(/branch is kept/)).toBeInTheDocument();
    expect(ipc.lanePreview).toHaveBeenCalledWith("t1", "remove");
    await userEvent.click(screen.getByRole("button", { name: "Remove lane" }));
    await waitFor(() => expect(ipc.laneRemove).toHaveBeenCalledWith("t1", false, false, true));
    expect(onClose).toHaveBeenCalled();
  });

  it("needs a second confirmation to discard uncommitted files", async () => {
    ipc.lanePreview.mockResolvedValue(preview({ changedFiles: ["wip.txt", "b.txt"], changedTotal: 2 }));
    render(<LaneDialog state={{ nodeId: "t1", action: "remove" }} onClose={() => {}} />);
    expect(await screen.findByText("wip.txt")).toBeInTheDocument();
    const remove = screen.getByRole("button", { name: "Remove lane" });
    expect(remove).toBeDisabled();
    await userEvent.click(screen.getByRole("checkbox", { name: /Discard these 2 uncommitted files/ }));
    await userEvent.click(remove);
    await waitFor(() => expect(ipc.laneRemove).toHaveBeenCalledWith("t1", true, false, true));
  });

  it("warns before deleting a branch with commits found nowhere else", async () => {
    ipc.lanePreview.mockResolvedValue(preview({ unmergedCommits: 3 }));
    render(<LaneDialog state={{ nodeId: "t1", action: "remove" }} onClose={() => {}} />);
    await userEvent.click(await screen.findByRole("checkbox", { name: /Also delete the branch/ }));
    expect(screen.getByText(/3 commits exist only on lane\/api/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Remove lane" }));
    await waitFor(() => expect(ipc.laneRemove).toHaveBeenCalledWith("t1", false, true, true));
  });

  it("deletes the terminal through the same choices when asked to", async () => {
    const { useCanvas } = await import("../src/state/canvas");
    const removeNode = vi.fn(async () => {});
    useCanvas.setState({ removeNode });
    render(<LaneDialog state={{ nodeId: "t1", action: "remove", deleteNode: true }} onClose={() => {}} />);
    await userEvent.click(await screen.findByRole("button", { name: "Delete terminal" }));
    await waitFor(() =>
      expect(removeNode).toHaveBeenCalledWith("t1", true, { discardChanges: false, deleteBranch: false }),
    );
    expect(ipc.laneRemove).not.toHaveBeenCalled();
  });

  it("offers removal when there is nothing to land", async () => {
    ipc.lanePreview.mockResolvedValue(preview({ commits: 0 }));
    render(<LaneDialog state={{ nodeId: "t1", action: "land" }} onClose={() => {}} />);
    await userEvent.click(await screen.findByRole("button", { name: "Remove lane…" }));
    expect(await screen.findByRole("button", { name: "Remove lane" })).toBeInTheDocument();
    expect(ipc.lanePreview).toHaveBeenLastCalledWith("t1", "remove");
  });
});

describe("Deleting a lane terminal", () => {
  it("opens the remove dialog instead of failing", async () => {
    const { useCanvas } = await import("../src/state/canvas");
    const nodeDelete = vi.fn(async () => {
      throw { code: "LaneConfirmationNeeded", message: "This terminal works in a lane." };
    });
    (ipc as Record<string, unknown>).nodeDelete = nodeDelete;
    useCanvas.setState(useCanvas.getInitialState(), true);
    useCanvas.setState({ workspaceId: "w1" });
    await useCanvas.getState().removeNode("t1", false);
    expect(useCanvas.getState().laneDialog).toEqual({ nodeId: "t1", action: "remove", deleteNode: true });
  });
});
