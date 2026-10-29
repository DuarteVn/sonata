import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { KimiIntegration } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  kimiIntegration: vi.fn(),
  kimiIntegrationApply: vi.fn(),
  kimiIntegrationRemove: vi.fn(),
  kimiWatch: vi.fn(async () => {}),
  usageState: vi.fn(async () => undefined as unknown),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));
vi.mock("../src/ipc/events", () => ({ onEvent: vi.fn(async () => () => {}) }));

import { AgentSettings } from "../src/settings/AgentSettings";
import { SettingsApp } from "../src/settings/SettingsApp";

const BLOCK = '# >>> Sonata integration (v1). >>>\n[[hooks]]\nevent = "Stop"\n# <<< Sonata integration <<<\n';

function integration(over: Partial<KimiIntegration> = {}): KimiIntegration {
  return {
    status: "off",
    configPath: "/Users/me/.kimi-code/config.toml",
    block: BLOCK,
    canApply: true,
    validated: true,
    ...over,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  ipc.kimiIntegration.mockResolvedValue(integration());
});

describe("Settings › Agents › Kimi Code", () => {
  it("is a section of the Settings window", async () => {
    render(<SettingsApp />);
    await userEvent.click(screen.getByRole("button", { name: /Agents/ }));
    expect(await screen.findByRole("heading", { name: "Kimi Code" })).toBeInTheDocument();
  });

  it("shows status, path, the exact lines, and what each hook does", async () => {
    render(<AgentSettings />);
    expect(await screen.findByText(/Off/)).toBeInTheDocument();
    expect(screen.getByText("/Users/me/.kimi-code/config.toml")).toBeInTheDocument();
    expect(screen.getByLabelText("Lines Sonata adds")).toHaveTextContent('event = "Stop"');
    for (const hook of ["SessionStart", "UserPromptSubmit", "Stop", "StopFailure", "Interrupt"]) {
      expect(screen.getByText(hook)).toBeInTheDocument();
    }
    expect(ipc.kimiWatch).toHaveBeenCalledWith(true);
  });

  it("applies only after confirmation and reports the backup", async () => {
    ipc.kimiIntegrationApply.mockResolvedValue(
      integration({ status: "on", lastBackup: "/Users/me/.kimi-code/config.toml.sonata-backup-20261026-120000" }),
    );
    render(<AgentSettings />);
    await userEvent.click(await screen.findByRole("button", { name: "Apply" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent(
      "Sonata will back up and change /Users/me/.kimi-code/config.toml: it adds only the lines shown. Kimi outside Sonata is not affected.",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(ipc.kimiIntegrationApply).not.toHaveBeenCalled();

    await userEvent.click(screen.getByRole("button", { name: "Apply" }));
    await userEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Apply" }));
    await waitFor(() => expect(ipc.kimiIntegrationApply).toHaveBeenCalledTimes(1));
    expect(
      await screen.findByText(
        "Backup saved to /Users/me/.kimi-code/config.toml.sonata-backup-20261026-120000. Restart running Kimi terminals to use the change.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/^On/)).toBeInTheDocument();
  });

  it("removes only after confirmation", async () => {
    ipc.kimiIntegration.mockResolvedValue(integration({ status: "on" }));
    ipc.kimiIntegrationRemove.mockResolvedValue(integration({ status: "off", lastBackup: "/b" }));
    render(<AgentSettings />);
    await userEvent.click(await screen.findByRole("button", { name: "Remove" }));
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent(
      "Sonata will back up /Users/me/.kimi-code/config.toml and remove only its own lines.",
    );
    await userEvent.click(within(dialog).getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(ipc.kimiIntegrationRemove).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole("button", { name: "Apply" })).toBeInTheDocument();
  });

  it("disables Apply and explains why when it cannot apply", async () => {
    ipc.kimiIntegration.mockResolvedValue(
      integration({ canApply: false, message: "Kimi's config defines `hooks` on line 2 (`hooks = []`)" }),
    );
    render(<AgentSettings />);
    expect(await screen.findByRole("button", { name: "Apply" })).toBeDisabled();
    expect(screen.getByText(/defines `hooks` on line 2/)).toBeInTheDocument();
  });

  it("shows errors from Apply", async () => {
    ipc.kimiIntegrationApply.mockRejectedValue({ code: "KimiCheckFailed", message: "Kimi rejected the changed file" });
    render(<AgentSettings />);
    await userEvent.click(await screen.findByRole("button", { name: "Apply" }));
    await userEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Apply" }));
    expect(await screen.findByText("Kimi rejected the changed file")).toBeInTheDocument();
  });

  it("copies the lines", async () => {
    const user = userEvent.setup();
    render(<AgentSettings />);
    await user.click(await screen.findByRole("button", { name: /Copy lines/ }));
    expect(await navigator.clipboard.readText()).toBe(BLOCK);
  });

  it("says when Kimi is not on PATH to validate the file", async () => {
    ipc.kimiIntegration.mockResolvedValue(integration({ validated: false }));
    render(<AgentSettings />);
    expect(await screen.findByText(/kimi doctor.*not found/i)).toBeInTheDocument();
  });
});
