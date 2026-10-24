import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageState } from "../src/ipc/types";
import { provider, reading, usageState } from "./usageFixtures";

const ipc = vi.hoisted(() => ({
  usageState: vi.fn(async () => undefined as unknown),
  usageRefresh: vi.fn(async () => undefined as unknown),
  usageSetEnabled: vi.fn(async () => undefined as unknown),
  usageSetWarnings: vi.fn(async () => undefined as unknown),
  usageSetProviderEnabled: vi.fn(async () => undefined as unknown),
  usageSetProviderColor: vi.fn(async () => undefined as unknown),
  usageProvidersFolder: vi.fn(async () => ({ path: "/Users/me/.sonata/usage-providers" })),
  appRevealPath: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { UsageSettings } from "../src/settings/UsageSettings";
import { useUsage } from "../src/state/usage";

const NOW = new Date("2026-10-23T12:00:00Z");

function show(state: UsageState) {
  useUsage.setState({ state });
  render(<UsageSettings now={NOW} />);
}

const state = () =>
  usageState([
    provider({ reading: reading({ readAt: "2026-10-23T11:57:00Z" }) }),
    provider({ id: "codex", name: "Codex", color: "#10a37f", detection: "notFound", commandPath: null, reading: null }),
    provider({
      id: "mine",
      name: "My Agent",
      kind: "custom",
      consent: "unasked",
      reading: null,
      definitionError: null,
      commandPath: "/Users/me/bin/usage",
    }),
    provider({
      id: "bad",
      name: "bad.json",
      kind: "custom",
      reading: null,
      definitionError: "command[0] must be an absolute path",
      commandPath: null,
    }),
  ]);

beforeEach(() => {
  vi.clearAllMocks();
  useUsage.setState(useUsage.getInitialState(), true);
});

describe("usage settings", () => {
  it("master switch and warnings switch call core", async () => {
    show(state());
    await userEvent.click(screen.getByRole("switch", { name: "Read agent usage" }));
    expect(ipc.usageSetEnabled).toHaveBeenCalledWith(false);
    await userEvent.click(screen.getByRole("switch", { name: "Warn at 80% and 100%" }));
    expect(ipc.usageSetWarnings).toHaveBeenCalledWith(false);
  });

  it("shows one row per agent with tag, path and status", () => {
    show(state());
    const claude = screen.getByRole("row", { name: /Claude Code/ });
    expect(claude).toHaveTextContent("Built-in");
    expect(claude).toHaveTextContent("/opt/homebrew/bin/claude");
    expect(claude).toHaveTextContent("Read 3 min ago");
    const codex = screen.getByRole("row", { name: /Codex/ });
    expect(codex).toHaveTextContent("Command not found");
    const mine = screen.getByRole("row", { name: /My Agent/ });
    expect(mine).toHaveTextContent("Custom");
    expect(mine).toHaveTextContent("Waiting for permission");
    expect(screen.getByRole("row", { name: /bad\.json/ })).toHaveTextContent("command[0] must be an absolute path");
  });

  it("per-agent switch, color and refresh", async () => {
    show(state());
    const claude = screen.getByRole("row", { name: /Claude Code/ });
    await userEvent.click(within(claude).getByRole("switch", { name: "Read Claude Code" }));
    expect(ipc.usageSetProviderEnabled).toHaveBeenCalledWith("claude", false);
    await userEvent.click(within(claude).getByRole("button", { name: "Refresh Claude Code" }));
    expect(ipc.usageRefresh).toHaveBeenCalledWith("manual", "claude");
    const color = within(claude).getByLabelText("Claude Code color");
    expect(color).toHaveValue("#d97757");
  });

  it("Refresh all reads every agent", async () => {
    show(state());
    await userEvent.click(screen.getByRole("button", { name: "Refresh all" }));
    expect(ipc.usageRefresh).toHaveBeenCalledWith("manual");
  });

  it("offers the providers folder", async () => {
    show(state());
    expect(screen.getByText("/Users/me/.sonata/usage-providers")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Show folder" }));
    expect(ipc.usageProvidersFolder).toHaveBeenCalled();
    expect(ipc.appRevealPath).toHaveBeenCalledWith("/Users/me/.sonata/usage-providers");
  });
});
