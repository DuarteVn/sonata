import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
const dialog = vi.hoisted(() => ({ open: vi.fn(async (): Promise<string | null> => null) }));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);
type DropHandler = (e: { payload: { type: string; paths?: string[] } }) => void;
const drop = vi.hoisted(() => ({ handler: null as DropHandler | null, unlisten: vi.fn() }));
vi.mock("@tauri-apps/api/webview", () => ({
  getCurrentWebview: () => ({
    onDragDropEvent: vi.fn(async (cb: DropHandler) => {
      drop.handler = cb;
      return drop.unlisten;
    }),
  }),
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

const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 1];
const JPEG = [0xff, 0xd8, 0xff, 0xe0, 2];

URL.createObjectURL = vi.fn(() => `blob:preview-${Math.random()}`);
URL.revokeObjectURL = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  handlers.clear();
  drop.handler = null;
  dialog.open.mockResolvedValue(null);
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

function pasteFile(target: Element, file: File) {
  fireEvent.paste(target, {
    clipboardData: {
      items: [{ kind: "file", type: file.type, getAsFile: () => file }],
      getData: () => "",
    },
  });
}

const preview = () => screen.queryByRole("img", { name: "Attached image" });

describe("attaching an image", () => {
  it("attaches a pasted PNG and sends its bytes", async () => {
    openDialog();
    await fill();
    pasteFile(screen.getByLabelText("What happened?"), new File([new Uint8Array(PNG)], "image.png", { type: "image/png" }));
    expect(await screen.findByRole("img", { name: "Attached image" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(ipc.reportSend).toHaveBeenCalledWith(
      expect.objectContaining({ image: { bytes: PNG, mime: "image/png" } }),
    );
  });

  it("pastes text as text", async () => {
    openDialog();
    await userEvent.click(screen.getByLabelText("What happened?"));
    await userEvent.paste("copied words");
    expect(screen.getByLabelText("What happened?")).toHaveValue("copied words");
    expect(preview()).toBeNull();
  });

  it("refuses a pasted image over 5 MB before reading it, keeping the earlier one", async () => {
    openDialog();
    pasteFile(document.body.querySelector(".report-dialog")!, new File([new Uint8Array(PNG)], "a.png", { type: "image/png" }));
    await screen.findByRole("img", { name: "Attached image" });
    const big = new File([new Uint8Array(PNG)], "big.png", { type: "image/png" });
    Object.defineProperty(big, "size", { value: 5 * 1024 * 1024 + 1 });
    const read = vi.spyOn(big, "arrayBuffer");
    pasteFile(screen.getByLabelText("What happened?"), big);
    expect(await screen.findByRole("alert")).toHaveTextContent(/5 MB/);
    expect(read).not.toHaveBeenCalled();
    expect(preview()).toBeInTheDocument();
  });

  it("refuses a pasted GIF", async () => {
    openDialog();
    pasteFile(screen.getByLabelText("What happened?"), new File(["GIF89a"], "a.gif", { type: "image/gif" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/PNG or JPEG/);
    expect(preview()).toBeNull();
  });

  it("attaches a chosen file through core and replaces the earlier image", async () => {
    openDialog();
    await fill();
    dialog.open.mockResolvedValueOnce("/pics/a.png").mockResolvedValueOnce("/pics/b.jpg");
    ipc.reportReadImage
      .mockResolvedValueOnce({ bytes: PNG, mime: "image/png" })
      .mockResolvedValueOnce({ bytes: JPEG, mime: "image/jpeg" });
    await userEvent.click(screen.getByRole("button", { name: "Attach image…" }));
    await screen.findByRole("img", { name: "Attached image" });
    expect(dialog.open).toHaveBeenCalledWith(
      expect.objectContaining({ filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }] }),
    );
    expect(ipc.reportReadImage).toHaveBeenCalledWith("/pics/a.png");
    await userEvent.click(screen.getByRole("button", { name: "Attach image…" }));
    await waitFor(() => expect(ipc.reportReadImage).toHaveBeenCalledWith("/pics/b.jpg"));
    await waitFor(() => expect(URL.revokeObjectURL).toHaveBeenCalled());
    expect(screen.getAllByRole("img", { name: "Attached image" })).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(ipc.reportSend).toHaveBeenCalledWith(
      expect.objectContaining({ image: { bytes: JPEG, mime: "image/jpeg" } }),
    );
  });

  it("shows why core rejected a file and keeps the earlier preview", async () => {
    openDialog();
    dialog.open.mockResolvedValueOnce("/pics/a.png").mockResolvedValueOnce("/pics/huge.png");
    ipc.reportReadImage
      .mockResolvedValueOnce({ bytes: PNG, mime: "image/png" })
      .mockRejectedValueOnce({ kind: "invalid", field: "image", reason: "too_large" });
    await userEvent.click(screen.getByRole("button", { name: "Attach image…" }));
    await screen.findByRole("img", { name: "Attached image" });
    await userEvent.click(screen.getByRole("button", { name: "Attach image…" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/5 MB/);
    expect(preview()).toBeInTheDocument();
  });

  it("attaches the first dropped file while open and stops listening when closed", async () => {
    const { unmount } = openDialog();
    ipc.reportReadImage.mockResolvedValueOnce({ bytes: JPEG, mime: "image/jpeg" });
    await waitFor(() => expect(drop.handler).not.toBeNull());
    act(() => drop.handler!({ payload: { type: "over" } }));
    act(() => drop.handler!({ payload: { type: "drop", paths: ["/d/one.jpg", "/d/two.png"] } }));
    expect(await screen.findByRole("img", { name: "Attached image" })).toBeInTheDocument();
    expect(ipc.reportReadImage).toHaveBeenCalledTimes(1);
    expect(ipc.reportReadImage).toHaveBeenCalledWith("/d/one.jpg");
    unmount();
    expect(drop.unlisten).toHaveBeenCalled();
  });

  it("removes the image", async () => {
    openDialog();
    pasteFile(screen.getByLabelText("What happened?"), new File([new Uint8Array(PNG)], "a.png", { type: "image/png" }));
    await screen.findByRole("img", { name: "Attached image" });
    await userEvent.click(screen.getByRole("button", { name: "Remove image" }));
    expect(preview()).toBeNull();
    expect(URL.revokeObjectURL).toHaveBeenCalled();
  });
});
