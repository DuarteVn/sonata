import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Lane, LaneStatus } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({ laneForget: vi.fn(async () => {}) }));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { LaneBadge } from "../src/nodes/LaneBadge";
import { TerminalMenu } from "../src/nodes/TerminalMenu";
import { useCanvas } from "../src/state/canvas";

const lane: Lane = { branch: "lane/api", baseBranch: "main", mainCheckout: "/r", checkoutPath: "/d/api", createdAt: "" };
const status = (over: Partial<LaneStatus> = {}): LaneStatus => ({
  nodeId: "t1",
  state: "ok",
  branch: "lane/api",
  baseBranch: "main",
  changed: 0,
  ahead: 0,
  behind: 0,
  ...over,
});

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
});

describe("LaneBadge", () => {
  it("shows the branch alone until a status arrives", () => {
    render(<LaneBadge lane={lane} />);
    expect(screen.getByText("lane/api")).toBeInTheDocument();
    expect(screen.queryByLabelText(/uncommitted/)).not.toBeInTheDocument();
  });

  it("shows changed files and ahead/behind counts", () => {
    render(<LaneBadge lane={lane} status={status({ changed: 3, ahead: 2, behind: 1 })} />);
    expect(screen.getByLabelText("3 uncommitted files")).toHaveTextContent("●3");
    expect(screen.getByLabelText("2 commits ahead of main")).toHaveTextContent("↑2");
    expect(screen.getByLabelText("1 commit behind main")).toHaveTextContent("↓1");
  });

  it("hides zero counts", () => {
    render(<LaneBadge lane={lane} status={status()} />);
    expect(screen.queryByLabelText(/uncommitted|ahead|behind/)).not.toBeInTheDocument();
  });

  it("explains broken states", () => {
    render(<LaneBadge lane={lane} status={status({ state: "checkoutMissing", changed: undefined })} />);
    expect(screen.getByText(/folder missing/)).toBeInTheDocument();
  });
});

describe("Forget lane", () => {
  it("is offered only when the lane folder is missing", async () => {
    const node = {
      id: "t1", frame: { x: 0, y: 0, width: 300, height: 200 }, z: 1, createdAt: "", modifiedAt: "",
      content: { terminal: { name: "Api", saveHistory: true, lane } },
    };
    useCanvas.setState({ laneStatuses: { t1: status({ state: "checkoutMissing" }) } });
    render(<TerminalMenu node={node} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminal options" }));
    expect(screen.queryByRole("menuitem", { name: /Land/ })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("menuitem", { name: "Forget lane" }));
    expect(ipc.laneForget).toHaveBeenCalledWith("t1");
  });
});
