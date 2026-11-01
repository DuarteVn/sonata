import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Bootstrap, ReportConfigView, UpdateState } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  appBootstrap: vi.fn(),
  updateState: vi.fn(async (): Promise<UpdateState> => ({ kind: "idle" })),
  terminalPreviewSubscribe: vi.fn(async () => {}),
  browserLayout: vi.fn(async () => {}),
  reportConfig: vi.fn(async (): Promise<ReportConfigView> => ({ serviceConfigured: true, fallbackEmail: "bugs@example.test" })),
  reportSend: vi.fn(async () => {}),
  reportReadImage: vi.fn(),
  reportFallback: vi.fn(),
}));
const handlers = vi.hoisted(() => new Map<string, (e: { payload: unknown }) => void>());
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, cb: (e: { payload: unknown }) => void) => {
    handlers.set(name, cb);
    return () => {};
  }),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(async () => null) }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({ onDragDropEvent: vi.fn(async () => () => {}) }),
}));

import { App } from "../src/App";
import { IMPROVEMENT_POINTS, ReportBugDialog } from "../src/report/ReportBugDialog";
import { clearDraft } from "../src/report/draft";
import { resetInitForTests, useWorkspaces } from "../src/state/workspaces";
import { useUpdate } from "../src/state/update";

const boot: Bootstrap = {
  workspaces: [],
  loadFailures: [],
  activeWorkspaceId: null,
  crashGuardTripped: false,
  recentWorkspaceIds: [],
  preferences: { autosaveIntervalSeconds: 30, terminalFontSize: 13, historyMaxRows: 10000 },
  warnings: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  handlers.clear();
  clearDraft();
  localStorage.clear();
  ipc.reportConfig.mockResolvedValue({ serviceConfigured: true, fallbackEmail: "bugs@example.test" });
  ipc.reportSend.mockResolvedValue(undefined);
});

function openDialog() {
  const onClose = vi.fn();
  const view = render(<ReportBugDialog onClose={onClose} />);
  return { onClose, ...view };
}

async function fill(name = "Ana", description = "The note vanished.") {
  if (name) await userEvent.type(screen.getByLabelText("Your name"), name);
  if (description) await userEvent.type(screen.getByLabelText("What happened?"), description);
}

describe("Report Bug dialog", () => {
  it("shows the headline exactly", () => {
    openDialog();
    expect(IMPROVEMENT_POINTS).toBe("Nosso sistema não tem bugs, tem PONTOS DE MELHORIA");
    expect(screen.getByRole("dialog", { name: "Report Bug" })).toHaveTextContent(
      "Nosso sistema não tem bugs, tem PONTOS DE MELHORIA",
    );
  });

  it("keeps Send disabled until name and description are filled", async () => {
    openDialog();
    const send = screen.getByRole("button", { name: "Send" });
    expect(send).toBeDisabled();
    await fill("Ana", "");
    expect(send).toBeDisabled();
    await userEvent.type(screen.getByLabelText("What happened?"), "   ");
    expect(send).toBeDisabled();
    await userEvent.type(screen.getByLabelText("What happened?"), "broke");
    expect(send).toBeEnabled();
    await userEvent.clear(screen.getByLabelText("Your name"));
    expect(send).toBeDisabled();
  });

  it("sends once, shows progress, then thanks and clears the form", async () => {
    let finish: () => void = () => {};
    ipc.reportSend.mockImplementation(() => new Promise<void>((resolve) => (finish = resolve)));
    const { onClose, unmount } = openDialog();
    await fill();
    const send = screen.getByRole("button", { name: "Send" });
    await userEvent.dblClick(send);
    expect(screen.getByRole("status")).toHaveTextContent("Sending…");
    expect(send).toBeDisabled();
    expect(ipc.reportSend).toHaveBeenCalledTimes(1);
    expect(ipc.reportSend).toHaveBeenCalledWith({
      name: "Ana",
      description: "The note vanished.",
      image: null,
      includeLogs: false,
    });
    await act(async () => finish());
    expect(screen.getByRole("status")).toHaveTextContent("Thanks! Your report was sent.");
    expect(onClose).not.toHaveBeenCalled();
    unmount();

    openDialog();
    expect(screen.getByLabelText("Your name")).toHaveValue("");
    expect(screen.getByLabelText("What happened?")).toHaveValue("");
  });

  it("keeps the draft when closed and reopened", async () => {
    const { onClose, unmount } = openDialog();
    await fill("Bo", "Half typed");
    await userEvent.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
    unmount();
    openDialog();
    expect(screen.getByLabelText("Your name")).toHaveValue("Bo");
    expect(screen.getByLabelText("What happened?")).toHaveValue("Half typed");
  });

  it("shows a counter from 9,000 characters", async () => {
    openDialog();
    const area = screen.getByLabelText("What happened?");
    await userEvent.click(area);
    await userEvent.paste("a".repeat(8_999));
    expect(screen.queryByText(/\/ 10,000/)).toBeNull();
    await userEvent.paste("b");
    expect(screen.getByText("9,000 / 10,000")).toBeInTheDocument();
    expect(area).toHaveAttribute("maxLength", "10000");
    expect(screen.getByLabelText("Your name")).toHaveAttribute("maxLength", "100");
  });
});

describe("opening the dialog", () => {
  beforeEach(() => {
    useWorkspaces.setState(useWorkspaces.getInitialState(), true);
    resetInitForTests();
    useUpdate.setState(useUpdate.getInitialState(), true);
    ipc.appBootstrap.mockResolvedValue(boot);
  });

  it("opens from Help → Report Bug…", async () => {
    render(<App />);
    await screen.findByRole("navigation", { name: "Workspaces" });
    await waitFor(() => expect(handlers.has("report-bug")).toBe(true));
    act(() => handlers.get("report-bug")!({ payload: null }));
    expect(await screen.findByRole("dialog", { name: "Report Bug" })).toBeInTheDocument();
  });

  it("opens from the sidebar Report Bug button", async () => {
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: "Report Bug" }));
    expect(screen.getByRole("dialog", { name: "Report Bug" })).toBeInTheDocument();
  });
});
