import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Bootstrap, ReportConfigView, UpdateState } from "../src/ipc/types";

const SERVICE = { serviceConfigured: true, reportEmail: "bugs@example.test", mailBodyRoom: 1_400 };
const NO_SERVICE = { ...SERVICE, serviceConfigured: false };

const ipc = vi.hoisted(() => ({
  appBootstrap: vi.fn(),
  updateState: vi.fn(async (): Promise<UpdateState> => ({ kind: "idle" })),
  terminalPreviewSubscribe: vi.fn(async () => {}),
  browserLayout: vi.fn(async () => {}),
  reportConfig: vi.fn<() => Promise<ReportConfigView>>(),
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
import { shortcut } from "../src/platform";
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
  ipc.reportConfig.mockResolvedValue(SERVICE);
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
    const send = await screen.findByRole("button", { name: "Send" });
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

describe("logs", () => {
  it("are off by default, explain the risk, and are sent only when ticked", async () => {
    openDialog();
    await fill();
    const box = screen.getByRole("checkbox", { name: /Include logs/ });
    expect(box).not.toBeChecked();
    expect(screen.getByText("Logs can contain folder paths and project names.")).toBeInTheDocument();
    await userEvent.click(box);
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
    expect(ipc.reportSend).toHaveBeenCalledWith(expect.objectContaining({ includeLogs: true }));
  });
});

describe("when sending fails", () => {
  const writeText = vi.fn(async () => {});
  beforeEach(() => {
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    writeText.mockClear();
  });

  async function failWith(error: unknown) {
    ipc.reportSend.mockRejectedValueOnce(error);
    openDialog();
    await fill();
    pasteFile(screen.getByLabelText("What happened?"), new File([new Uint8Array(PNG)], "a.png", { type: "image/png" }));
    await screen.findByRole("img", { name: "Attached image" });
    await userEvent.click(screen.getByRole("button", { name: "Send" }));
  }

  it.each([
    [{ kind: "unreachable" }, "Could not reach the report service. Check your connection and try again."],
    [{ kind: "rate_limited", retryAfterSecs: 120 }, "Too many reports right now. Please try again later."],
    [{ kind: "refused", code: "invalid_field:name" }, "The report service did not accept this report."],
    ["boom", "Could not reach the report service. Check your connection and try again."],
  ])("explains %o, keeps the report, and retries", async (error, message) => {
    await failWith(error);
    expect(await screen.findByRole("status")).toHaveTextContent(message);
    expect(screen.getByLabelText("Your name")).toHaveValue("Ana");
    expect(screen.getByLabelText("What happened?")).toHaveValue("The note vanished.");
    expect(preview()).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(ipc.reportSend).toHaveBeenCalledTimes(2);
    expect(await screen.findByRole("status")).toHaveTextContent("Thanks! Your report was sent.");
  });

  it("opens the mail app as a fallback", async () => {
    ipc.reportFallback.mockResolvedValueOnce({ opened: true, text: "full text", truncated: false });
    await failWith({ kind: "unreachable" });
    await userEvent.click(await screen.findByRole("button", { name: "Send by email instead" }));
    expect(ipc.reportFallback).toHaveBeenCalledWith(
      expect.objectContaining({ name: "Ana", image: { bytes: PNG, mime: "image/png" } }),
    );
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Your mail app opened. Attach the image by hand if you have one.",
    );
    expect(writeText).not.toHaveBeenCalled();
  });

  it.each([
    [{ opened: false, text: "full text", truncated: false }],
    [{ opened: true, text: "full text", truncated: true }],
  ])("copies the report when the mail link is missing or cut (%o)", async (result) => {
    ipc.reportFallback.mockResolvedValueOnce(result);
    await failWith({ kind: "unreachable" });
    await userEvent.click(await screen.findByRole("button", { name: "Send by email instead" }));
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("full text"));
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Report copied to the clipboard. Paste it in a message to the maintainer.",
    );
  });
});

describe("when no report service is configured", () => {
  type Written = { type: string; blob: Blob };
  const order: string[] = [];
  const written: Written[] = [];
  const write = vi.fn();
  const writeText = vi.fn();

  class FakeClipboardItem {
    constructor(readonly parts: Record<string, Promise<Blob>>) {}
  }

  const OPENED = { opened: true, text: "full text", truncated: false };
  const MAIL_ONLY = "Your mail app opened with the report.";

  beforeEach(() => {
    order.length = 0;
    written.length = 0;
    ipc.reportConfig.mockResolvedValue(NO_SERVICE);
    ipc.reportFallback.mockReset();
    ipc.reportFallback.mockImplementation(async () => {
      order.push("mail");
      return OPENED;
    });
    write.mockReset();
    write.mockImplementation(async ([item]: FakeClipboardItem[]) => {
      order.push("clipboard write");
      for (const [type, part] of Object.entries(item.parts)) written.push({ type, blob: await part });
    });
    writeText.mockReset();
    writeText.mockResolvedValue(undefined);
    vi.stubGlobal("ClipboardItem", FakeClipboardItem);
    Object.defineProperty(navigator, "clipboard", { value: { write, writeText }, configurable: true });
  });

  afterEach(() => vi.unstubAllGlobals());

  const openEmail = () => screen.findByRole("button", { name: "Open email" });

  async function press() {
    await userEvent.click(await openEmail());
  }

  async function attach(file = new File([new Uint8Array(PNG)], "a.png", { type: "image/png" })) {
    pasteFile(screen.getByLabelText("What happened?"), file);
    await screen.findByRole("img", { name: "Attached image" });
  }

  const longDescription = "x".repeat(NO_SERVICE.mailBodyRoom);

  async function fillLong() {
    await userEvent.type(screen.getByLabelText("Your name"), "Ana");
    await userEvent.click(screen.getByLabelText("What happened?"));
    await userEvent.paste(longDescription);
  }

  it("offers Open email, hides the logs option, and needs the required fields", async () => {
    openDialog();
    const button = await openEmail();
    expect(button).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Send" })).toBeNull();
    expect(screen.queryByRole("checkbox", { name: /Include logs/ })).toBeNull();
    await fill();
    expect(button).toBeEnabled();
  });

  it("opens the mail app with the report, without an error or a clipboard change", async () => {
    openDialog();
    await fill();
    await press();
    expect(await screen.findByRole("status")).toHaveTextContent(MAIL_ONLY);
    expect(ipc.reportFallback).toHaveBeenCalledTimes(1);
    expect(ipc.reportFallback).toHaveBeenCalledWith({
      name: "Ana",
      description: "The note vanished.",
      image: null,
      includeLogs: false,
    });
    expect(ipc.reportSend).not.toHaveBeenCalled();
    expect(screen.getByRole("status")).not.toHaveTextContent("cannot send");
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Send by email instead" })).toBeNull();
    await waitFor(() => expect(write).toHaveBeenCalled());
    expect(written).toEqual([]);
    expect(writeText).not.toHaveBeenCalled();
  });

  it("opens the mail app once on a double click and keeps the draft", async () => {
    openDialog();
    await fill();
    await userEvent.dblClick(await openEmail());
    await screen.findByText(MAIL_ONLY);
    expect(ipc.reportFallback).toHaveBeenCalledTimes(1);
    expect(screen.getByLabelText("What happened?")).toHaveValue("The note vanished.");
  });

  it("copies a PNG unchanged before the mail app opens", async () => {
    openDialog();
    await fill();
    await attach();
    await press();
    expect(await screen.findByRole("status")).toHaveTextContent(
      `Your mail app opened with the report. The image is in the clipboard: paste it into the email (${shortcut("paste")}).`,
    );
    expect(order).toEqual(["clipboard write", "mail"]);
    expect(written.map((w) => w.type)).toEqual(["image/png"]);
    expect([...new Uint8Array(await written[0].blob.arrayBuffer())]).toEqual(PNG);
    expect(ipc.reportFallback).toHaveBeenCalledWith(expect.objectContaining({ image: null }));
  });

  it("starts the clipboard write inside the click, before anything is awaited", async () => {
    // WebKit refuses clipboard writes made after the click's task ends.
    openDialog();
    await fill();
    await attach();
    const button = await openEmail();
    fireEvent.submit(button.closest("form")!);
    expect(write).toHaveBeenCalledTimes(1);
    await screen.findByText(/paste it into the email/);
  });

  it("converts a JPEG to PNG through a canvas", async () => {
    const close = vi.fn();
    vi.stubGlobal("createImageBitmap", vi.fn(async () => ({ width: 3, height: 2, close })));
    const drawImage = vi.fn();
    const png = new Blob([new Uint8Array(PNG)], { type: "image/png" });
    const getContext = vi
      .spyOn(HTMLCanvasElement.prototype, "getContext")
      .mockReturnValue({ drawImage } as unknown as ReturnType<HTMLCanvasElement["getContext"]>);
    const toBlob = vi
      .spyOn(HTMLCanvasElement.prototype, "toBlob")
      .mockImplementation((done: BlobCallback) => done(png));
    openDialog();
    await fill();
    await attach(new File([new Uint8Array(JPEG)], "a.jpg", { type: "image/jpeg" }));
    await press();
    await screen.findByText(/paste it into the email/);
    expect(written.map((w) => w.type)).toEqual(["image/png"]);
    expect(written[0].blob).toBe(png);
    expect(drawImage).toHaveBeenCalledTimes(1);
    expect(toBlob).toHaveBeenCalledWith(expect.any(Function), "image/png");
    expect(close).toHaveBeenCalled();
    getContext.mockRestore();
    toBlob.mockRestore();
  });

  it("says to attach by hand when the image cannot be copied", async () => {
    write.mockRejectedValue(new Error("not allowed"));
    openDialog();
    await fill();
    await attach();
    await press();
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Your mail app opened with the report. Attach the image by hand.",
    );
    expect(ipc.reportFallback).toHaveBeenCalledTimes(1);
  });

  it("says to attach by hand when the clipboard API is missing", async () => {
    vi.stubGlobal("ClipboardItem", undefined);
    openDialog();
    await fill();
    await attach();
    await press();
    expect(await screen.findByRole("status")).toHaveTextContent(
      "Your mail app opened with the report. Attach the image by hand.",
    );
  });

  it("copies the full text instead of the image when the body is cut", async () => {
    ipc.reportFallback.mockResolvedValue({ opened: true, text: "the whole report", truncated: true });
    openDialog();
    await fillLong();
    await attach();
    await press();
    expect(await screen.findByRole("status")).toHaveTextContent(
      "The report was long, so the full text is in the clipboard. Paste it into the email and attach the image by hand.",
    );
    expect(write).toHaveBeenCalledTimes(1);
    expect(written.map((w) => w.type)).toEqual(["text/plain"]);
    expect(await written[0].blob.text()).toBe("the whole report");
  });

  it("does not mention an image in a long report that has none", async () => {
    ipc.reportFallback.mockResolvedValue({ opened: true, text: "the whole report", truncated: true });
    openDialog();
    await fillLong();
    await press();
    const status = await screen.findByRole("status");
    await waitFor(() =>
      expect(status).toHaveTextContent("The report was long, so the full text is in the clipboard. Paste it into the email."),
    );
    expect(status).not.toHaveTextContent("image");
  });

  it("copies the text and shows the address when the mail app cannot open", async () => {
    ipc.reportFallback.mockResolvedValue({ opened: false, text: "the whole report", truncated: false });
    openDialog();
    await fill();
    await press();
    const status = await screen.findByRole("status");
    await waitFor(() =>
      expect(status).toHaveTextContent(
        "Report copied to the clipboard. Paste it in a message to the maintainer. The address is bugs@example.test.",
      ),
    );
    expect(written.map((w) => w.type)).toEqual(["text/plain"]);
    expect(await written[0].blob.text()).toBe("the whole report");
  });

  it("replaces the copied image with the text when the mail app cannot open", async () => {
    ipc.reportFallback.mockResolvedValue({ opened: false, text: "the whole report", truncated: false });
    openDialog();
    await fill();
    await attach();
    await press();
    await screen.findByText(/The address is bugs@example.test/);
    expect(writeText).toHaveBeenCalledWith("the whole report");
  });

  it("still shows the address when the text cannot be copied either", async () => {
    ipc.reportFallback.mockResolvedValue({ opened: false, text: "the whole report", truncated: false });
    write.mockRejectedValue(new Error("not allowed"));
    writeText.mockRejectedValue(new Error("not allowed"));
    openDialog();
    await fill();
    await press();
    expect(await screen.findByText(/The address is bugs@example.test/)).toHaveTextContent("Could not copy the report.");
  });
});
