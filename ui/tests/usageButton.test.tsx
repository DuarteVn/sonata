import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UsageSummary } from "../src/ipc/types";
import { provider, usageState } from "./usageFixtures";

vi.mock("../src/ipc/commands", () => ({ ipc: { usageRefresh: vi.fn(async () => undefined) } }));

import { UsageButton } from "../src/usage/UsageButton";
import { useUsage } from "../src/state/usage";

function show(summary: UsageSummary, enabled = true) {
  useUsage.setState({ state: usageState([provider()], { summary, enabled }) });
  render(<UsageButton />);
  return screen.getByRole("button", { name: /^Usage/ });
}

const base: UsageSummary = { state: "normal", percent: 42, providerId: "claude", windowKind: "short", resetsAt: null };

beforeEach(() => useUsage.setState(useUsage.getInitialState(), true));

describe("usage button", () => {
  it("shows the highest share with a ring in the agent color", () => {
    const button = show(base);
    expect(button).toHaveClass("normal");
    expect(button).toHaveTextContent("42%");
    const ring = screen.getByTestId("usage-ring");
    expect(ring).toHaveAttribute("stroke", "#d97757");
    expect(button.title).toMatch(/Claude Code: 42% of the short window limit/);
  });

  it("warns at 80% and shows the full state at 100%", () => {
    expect(show({ ...base, state: "warning", percent: 85 })).toHaveClass("warning");
  });

  it("shows the full state at 100%", () => {
    const button = show({ ...base, state: "full", percent: 100, windowKind: "weekly" });
    expect(button).toHaveClass("full");
    expect(button.title).toMatch(/weekly limit reached/);
  });

  it("shows usage as off", () => {
    const button = show({ state: "off", percent: null, providerId: null, windowKind: null, resetsAt: null }, false);
    expect(button).toHaveClass("off");
    expect(button).toHaveAccessibleName("Usage (off)");
    expect(button).not.toHaveTextContent("%");
  });

  it("shows no number when nothing was read", () => {
    const button = show({ state: "empty", percent: null, providerId: null, windowKind: null, resetsAt: null });
    expect(button).toHaveClass("empty");
    expect(screen.queryByTestId("usage-ring")).toBeNull();
  });
});
