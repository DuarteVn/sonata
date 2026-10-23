import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CanvasNode, ToolReport } from "../src/ipc/types";

const missing: ToolReport = {
  android: {
    sdk: null,
    looked: ["/Users/me/Library/Android/sdk"],
    avdHome: "/Users/me/.android/avd",
    avds: [],
    helpUrl: "https://developer.android.com/studio/install",
    avdHelpUrl: "https://developer.android.com/studio/run/managing-avds",
  },
  ios: { developerDir: null, helpUrl: "https://developer.apple.com/xcode/" },
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
}));
const dialog = vi.hoisted(() => ({ open: vi.fn(async () => "/chosen/sdk") }));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("@tauri-apps/plugin-dialog", () => dialog);
vi.mock("@tauri-apps/api/core", () => ({ convertFileSrc: (p: string) => `asset://${p}` }));

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
