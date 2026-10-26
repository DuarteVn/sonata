import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { KimiIntegration } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  kimiIntegration: vi.fn(),
  dismissKimiNotice: vi.fn(async () => {}),
  kimiOpenSettings: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { KimiNotice, isKimiCommand } from "../src/nodes/KimiNotice";
import { useKimi } from "../src/state/kimi";

function integration(status: KimiIntegration["status"]): KimiIntegration {
  return { status, configPath: "/k/config.toml", block: "", canApply: true, validated: true };
}

beforeEach(() => {
  vi.clearAllMocks();
  useKimi.setState(useKimi.getInitialState(), true);
  ipc.kimiIntegration.mockResolvedValue(integration("off"));
});

describe("Kimi notice on terminal nodes", () => {
  it("only Kimi launch commands count", () => {
    expect(isKimiCommand("kimi")).toBe(true);
    expect(isKimiCommand("kimi --yolo")).toBe(true);
    expect(isKimiCommand("claude")).toBe(false);
    expect(isKimiCommand("kimi-cli")).toBe(false);
    expect(isKimiCommand(undefined)).toBe(false);
  });

  it("shows the off line with Set up and Dismiss", async () => {
    render(<KimiNotice />);
    expect(
      await screen.findByText("Kimi: busy/done tracking and conversation restore are off."),
    ).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Set up" }));
    expect(ipc.kimiOpenSettings).toHaveBeenCalled();
  });

  it("says out of date for an old block", async () => {
    ipc.kimiIntegration.mockResolvedValue(integration("outdated"));
    render(<KimiNotice />);
    expect(
      await screen.findByText("Kimi: busy/done tracking and conversation restore are out of date."),
    ).toBeInTheDocument();
  });

  it("is hidden when the integration is on", async () => {
    ipc.kimiIntegration.mockResolvedValue(integration("on"));
    render(<KimiNotice />);
    await waitFor(() => expect(ipc.kimiIntegration).toHaveBeenCalled());
    expect(screen.queryByRole("button", { name: "Set up" })).not.toBeInTheDocument();
  });

  it("Dismiss hides it for that status and it returns when the status changes", async () => {
    render(<KimiNotice />);
    await userEvent.click(await screen.findByRole("button", { name: "Dismiss" }));
    expect(ipc.dismissKimiNotice).toHaveBeenCalledWith("off");
    expect(screen.queryByRole("button", { name: "Set up" })).not.toBeInTheDocument();
    useKimi.getState().set(integration("outdated"));
    expect(await screen.findByRole("button", { name: "Set up" })).toBeInTheDocument();
  });

  it("a dismissal saved in preferences hides it", async () => {
    useKimi.setState({ dismissed: "off" });
    render(<KimiNotice />);
    await waitFor(() => expect(ipc.kimiIntegration).toHaveBeenCalled());
    expect(screen.queryByRole("button", { name: "Set up" })).not.toBeInTheDocument();
  });
});
