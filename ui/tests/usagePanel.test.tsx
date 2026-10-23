import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageState } from "../src/ipc/types";
import { provider, reading, usageState } from "./usageFixtures";

const ipc = vi.hoisted(() => ({
  usageRefresh: vi.fn(async () => undefined as unknown),
  usageSetEnabled: vi.fn(async () => undefined as unknown),
  usageSetProviderEnabled: vi.fn(async () => undefined as unknown),
  usageOpenSettings: vi.fn(async () => {}),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { UsagePanel } from "../src/usage/UsagePanel";
import { useUsage } from "../src/state/usage";

function show(state: UsageState) {
  useUsage.setState({ state });
  ipc.usageRefresh.mockResolvedValue(state);
  render(<UsagePanel now={new Date("2026-10-23T12:00:00Z")} />);
}

beforeEach(() => {
  vi.clearAllMocks();
  useUsage.setState(useUsage.getInitialState(), true);
});

describe("usage panel", () => {
  it("shows plan, account and both windows", () => {
    show(usageState([provider()]));
    const section = screen.getByRole("region", { name: "Claude Code" });
    expect(section).toHaveTextContent("Team");
    expect(section).toHaveTextContent("Example Org · user@example.com");
    expect(section).toHaveTextContent("5-hour");
    expect(section).toHaveTextContent("76%");
    expect(section).toHaveTextContent("resets in 2 h 10 min");
    expect(section).toHaveTextContent("Weekly");
    expect(section).toHaveTextContent("13%");
    expect(screen.getByRole("meter", { name: "Claude Code 5-hour" })).toHaveAttribute("aria-valuenow", "76");
  });

  it("reads on open when the cache is old", () => {
    show(usageState([provider()]));
    expect(ipc.usageRefresh).toHaveBeenCalledWith("panelOpen");
  });

  it("refresh reads every agent now", async () => {
    show(usageState([provider()]));
    await userEvent.click(screen.getByRole("button", { name: "Refresh usage" }));
    expect(ipc.usageRefresh).toHaveBeenCalledWith("manual");
  });

  it("shows not logged in with the hint and no percentages", () => {
    show(
      usageState([
        provider({
          reading: reading({ status: "notLoggedIn", windows: [], message: "Run `claude auth login` in a terminal." }),
        }),
      ]),
    );
    const section = screen.getByRole("region", { name: "Claude Code" });
    expect(section).toHaveTextContent("Not logged in");
    expect(section).toHaveTextContent("claude auth login");
    expect(section).not.toHaveTextContent("%");
  });

  it("shows no plan limits", () => {
    show(usageState([provider({ reading: reading({ status: "noPlanLimits", windows: [] }) })]));
    expect(screen.getByRole("region", { name: "Claude Code" })).toHaveTextContent("No plan limits");
  });

  it("shows why usage is not available", () => {
    show(usageState([provider({ reading: null, lastError: "timed out after 20 s" })]));
    const section = screen.getByRole("region", { name: "Claude Code" });
    expect(section).toHaveTextContent("Usage not available");
    expect(section).toHaveTextContent("timed out after 20 s");
  });

  it("marks stale values with the last reading time", () => {
    show(usageState([provider({ stale: true, lastError: "offline" })]));
    const section = screen.getByRole("region", { name: "Claude Code" });
    expect(section).toHaveTextContent(/Stale · last read/);
    expect(section).toHaveTextContent("76%");
  });

  it("shows a reached limit", () => {
    const r = reading();
    r.windows![0].usedPercent = 100;
    show(usageState([provider({ reading: r })]));
    expect(screen.getByRole("region", { name: "Claude Code" })).toHaveTextContent("Limit reached");
  });

  it("lists the agents it can read when none is installed", () => {
    show(
      usageState(
        [
          provider({ detection: "notFound", reading: null }),
          provider({ id: "codex", name: "Codex", detection: "notFound", reading: null }),
        ],
        { summary: { state: "empty", percent: null, providerId: null, windowKind: null, resetsAt: null } },
      ),
    );
    expect(screen.getByText(/No agent with plan limits was found/)).toBeInTheDocument();
    expect(screen.getByText(/Claude Code, Codex/)).toBeInTheDocument();
  });

  it("asks for permission for an agent that was not allowed yet", async () => {
    show(usageState([provider({ consent: "unasked", reading: null })]));
    await userEvent.click(screen.getByRole("button", { name: "Review permission" }));
    expect(ipc.usageSetProviderEnabled).toHaveBeenCalledWith("claude", true);
  });

  it("shows a spinner while refreshing", () => {
    show(usageState([provider({ refreshing: true })]));
    expect(screen.getByRole("region", { name: "Claude Code" })).toHaveAttribute("aria-busy", "true");
  });

  it("offers to turn reading on when it is off", async () => {
    show(usageState([provider()], { enabled: false }));
    await userEvent.click(screen.getByRole("button", { name: "Turn on" }));
    expect(ipc.usageSetEnabled).toHaveBeenCalledWith(true);
  });
});
