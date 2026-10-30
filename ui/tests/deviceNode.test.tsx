import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode, ToolReport } from "../src/ipc/types";

const missing: ToolReport = {
  android: {
    sdk: null,
    looked: ["/Users/me/Library/Android/sdk"],
    avdHome: "/Users/me/.android/avd",
    avds: [],
    helpUrl: "https://developer.android.com/studio#command-line-tools-only",
    avdHelpUrl: "https://developer.android.com/tools/avdmanager",
  },
  ios: { developerDir: null, simulators: [], helpUrl: "https://developer.apple.com/xcode/" },
};
const noAvd: ToolReport = { ...missing, android: { ...missing.android, sdk: "/sdk", looked: ["/sdk"] } };
const ready: ToolReport = {
  ...missing,
  android: {
    ...noAvd.android,
    avds: [{ name: "Pixel_8_API_34", displayName: "Pixel 8", apiLevel: 34, osVersion: "Android 14", screen: [1080, 2400] }],
  },
};

const ipc = vi.hoisted(() => ({
  deviceTools: vi.fn(),
  deviceChooseSdk: vi.fn(),
  deviceSelect: vi.fn(),
  deviceOpenHelp: vi.fn(async () => {}),
  deviceState: vi.fn(),
  deviceBoot: vi.fn(async () => {}),
  deviceStop: vi.fn(async () => {}),
  deviceStream: vi.fn(async () => {}),
  deviceUnstream: vi.fn(async () => "/data/run/device/d2.png"),
  deviceFrameAck: vi.fn(async () => {}),
  deviceInput: vi.fn(async () => {}),
}));
const dialog = vi.hoisted(() => ({ open: vi.fn(async () => "/chosen/sdk") }));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: (p: string) => `asset://${p}`,
  Channel: class {
    onmessage: ((m: unknown) => void) | null = null;
  },
}));

import { DeviceNode } from "../src/nodes/DeviceNode";
import { routeKey } from "../src/canvas/keyboard";
import { useCanvas } from "../src/state/canvas";

const node: CanvasNode = {
  id: "d1",
  frame: { x: 0, y: 0, width: 380, height: 820 },
  z: 1,
  createdAt: "",
  modifiedAt: "",
  content: { device: { platform: "android", deviceId: "" } },
};

beforeEach(() => {
  vi.clearAllMocks();
  useCanvas.setState(useCanvas.getInitialState(), true);
  useCanvas.setState({ nodes: [node], workspaceId: "w" });
});

describe("device node picker (US2)", () => {
  it("D picks the Device tool", () => {
    expect(routeKey({ key: "d", textFocus: false, terminalFocus: false, hasSelection: false, linking: false })).toBe("tool:device");
  });

  it("says the SDK is missing, where Sonata looked, and links to the official page", async () => {
    ipc.deviceTools.mockResolvedValue(missing);
    render(<DeviceNode node={node} />);
    const android = await screen.findByRole("region", { name: "Android" });
    expect(within(android).getByText("Android SDK not found")).toBeInTheDocument();
    expect(within(android).getByText("/Users/me/Library/Android/sdk")).toBeInTheDocument();
    await userEvent.click(within(android).getByRole("button", { name: "Open install instructions" }));
    expect(ipc.deviceOpenHelp).toHaveBeenCalledWith("android");
    const ios = screen.getByRole("region", { name: "iOS" });
    expect(within(ios).getByText("Xcode not found")).toBeInTheDocument();
    await userEvent.click(within(ios).getByRole("button", { name: "Open Xcode page" }));
    expect(ipc.deviceOpenHelp).toHaveBeenCalledWith("xcode");
  });

  it("hides iOS where simulators cannot exist (Linux, Windows)", async () => {
    ipc.deviceTools.mockResolvedValue({ ...ready, ios: null });
    render(<DeviceNode node={node} />);
    await screen.findByRole("region", { name: "Android" });
    await screen.findByText("Pixel 8");
    expect(screen.queryByRole("region", { name: "iOS" })).not.toBeInTheDocument();
  });

  it("chooses an SDK folder and checks again", async () => {
    ipc.deviceTools.mockResolvedValue(missing);
    ipc.deviceChooseSdk.mockResolvedValue(noAvd);
    render(<DeviceNode node={node} />);
    await userEvent.click(await screen.findByRole("button", { name: "Choose SDK folder…" }));
    expect(dialog.open).toHaveBeenCalledWith(expect.objectContaining({ directory: true }));
    expect(ipc.deviceChooseSdk).toHaveBeenCalledWith("/chosen/sdk");
    expect(await screen.findByText("No virtual devices yet")).toBeInTheDocument();
    expect(screen.getByText(/\/Users\/me\/\.android\/avd/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "How to create one" }));
    expect(ipc.deviceOpenHelp).toHaveBeenCalledWith("avd");

    ipc.deviceTools.mockResolvedValue(ready);
    await userEvent.click(screen.getByRole("button", { name: "Check again" }));
    expect(await screen.findByRole("button", { name: /Pixel 8/ })).toBeInTheDocument();
  });

  it("lists virtual devices with their version and binds the node to the one chosen", async () => {
    ipc.deviceTools.mockResolvedValue(ready);
    ipc.deviceSelect.mockResolvedValue({
      ...node,
      content: { device: { platform: "android", deviceId: "Pixel_8_API_34", name: "Pixel 8", osVersion: "Android 14" } },
    });
    render(<DeviceNode node={node} />);
    const pick = await screen.findByRole("button", { name: "Pixel 8 Android 14" });
    await userEvent.click(pick);
    expect(ipc.deviceSelect).toHaveBeenCalledWith("d1", "android", "Pixel_8_API_34", "Pixel 8", "Android 14");
    expect(useCanvas.getState().nodes[0].content).toEqual({
      device: { platform: "android", deviceId: "Pixel_8_API_34", name: "Pixel 8", osVersion: "Android 14" },
    });
  });
});

const chosen: CanvasNode = {
  ...node,
  id: "d2",
  content: { device: { platform: "android", deviceId: "Pixel_8_API_34", name: "Pixel 8", osVersion: "Android 14" } },
};
const state = (kind: "off" | "booting" | "ready" | "stopped" | "error", message?: string) => ({
  nodeId: "d2",
  state: { kind, message },
  startedBySonata: true,
  reduced: false,
  screen: [1080, 2400] as [number, number],
});

describe("iOS simulators (US4)", () => {
  it("lists simulators apart from Android and binds the node to one", async () => {
    ipc.deviceTools.mockResolvedValue({
      ...ready,
      ios: {
        developerDir: "/Applications/Xcode.app/Contents/Developer",
        simulators: [{ udid: "AAAA-1111", name: "iPhone 16", osVersion: "iOS 18.2", booted: false }],
        helpUrl: missing.ios!.helpUrl,
      },
    });
    ipc.deviceSelect.mockResolvedValue({ ...node, content: { device: { platform: "ios", deviceId: "AAAA-1111", name: "iPhone 16" } } });
    render(<DeviceNode node={node} />);
    const ios = await screen.findByRole("region", { name: "iOS" });
    await userEvent.click(within(ios).getByRole("button", { name: "iPhone 16 iOS 18.2" }));
    expect(ipc.deviceSelect).toHaveBeenCalledWith("d1", "ios", "AAAA-1111", "iPhone 16", "iOS 18.2");
    expect(within(screen.getByRole("region", { name: "Android" })).getByRole("button", { name: /Pixel 8/ })).toBeInTheDocument();
  });

  it("is view only: keys are disabled and a note explains why", async () => {
    const sim: CanvasNode = { ...node, id: "d3", content: { device: { platform: "ios", deviceId: "AAAA-1111", name: "iPhone 16" } } };
    useCanvas.setState({ nodes: [sim] });
    ipc.deviceState.mockResolvedValue({ nodeId: "d3", state: { kind: "ready" }, startedBySonata: true, reduced: true, screen: [1179, 2556] });
    render(<DeviceNode node={sim} />);
    expect(await screen.findByText(/View only/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Back" })).toBeDisabled();
  });
});

describe("device node with a chosen device (US1)", () => {
  beforeEach(() => {
    useCanvas.setState({ nodes: [chosen] });
  });

  it("after a restart offers Boot and does not boot by itself (AS6)", async () => {
    ipc.deviceState.mockResolvedValue(state("off"));
    render(<DeviceNode node={chosen} />);
    await userEvent.click(await screen.findByRole("button", { name: "Boot" }));
    expect(ipc.deviceBoot).toHaveBeenCalledTimes(1);
    expect(ipc.deviceBoot).toHaveBeenCalledWith("d2");
  });

  it("shows boot progress, errors, and a stopped device", async () => {
    ipc.deviceState.mockResolvedValue(state("booting"));
    render(<DeviceNode node={chosen} />);
    expect(await screen.findByText("Booting…")).toBeInTheDocument();
    act(() => useCanvas.getState().setDeviceState(state("error", "HVF is not available")));
    expect(screen.getByRole("alert")).toHaveTextContent("HVF is not available");
    act(() => useCanvas.getState().setDeviceState(state("stopped")));
    expect(screen.getByText("Device stopped")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Boot" })).toBeInTheDocument();
  });

  it("sends Back, Home, and Recent apps, and stops", async () => {
    ipc.deviceState.mockResolvedValue(state("ready"));
    render(<DeviceNode node={chosen} />);
    for (const [label, key] of [["Back", "back"], ["Home", "home"], ["Recent apps", "recents"]]) {
      await userEvent.click(await screen.findByRole("button", { name: label }));
      expect(ipc.deviceInput).toHaveBeenLastCalledWith("d2", { key });
    }
    await userEvent.click(screen.getByRole("button", { name: "Stop" }));
    expect(ipc.deviceStop).toHaveBeenCalledWith("d2");
  });

  it("shows which terminal controls it (FR-016)", async () => {
    ipc.deviceState.mockResolvedValue(state("ready"));
    const agent: CanvasNode = { ...node, id: "t1", content: { terminal: { name: "Tester", saveHistory: true } } };
    useCanvas.setState({ nodes: [chosen, agent], asks: { k: { from: "t1", to: "d2" } } });
    render(<DeviceNode node={chosen} />);
    expect(await screen.findByText("controlled by Tester")).toBeInTheDocument();
  });

  it("streams only while live and shows the last image as a card otherwise", async () => {
    ipc.deviceState.mockResolvedValue(state("ready"));
    render(<DeviceNode node={chosen} />);
    expect(await screen.findByRole("img", { name: "Last screen of Pixel 8" })).toHaveAttribute(
      "src",
      expect.stringContaining("asset:///data/run/device/d2.png"),
    );
    expect(ipc.deviceStream).not.toHaveBeenCalled();
    act(() => useCanvas.getState().setDeviceLive(new Set(["d2"])));
    await vi.waitFor(() => expect(ipc.deviceStream).toHaveBeenCalled());
    const [id, width, height] = ipc.deviceStream.mock.calls[0] as unknown as [string, number, number];
    expect(id).toBe("d2");
    expect(width / height).toBeCloseTo(1080 / 2400, 1);
    act(() => useCanvas.getState().setDeviceLive(new Set()));
    await vi.waitFor(() => expect(ipc.deviceUnstream).toHaveBeenCalledTimes(2));
  });
});
