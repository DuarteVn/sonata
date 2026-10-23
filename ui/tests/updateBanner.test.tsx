import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { UpdateState } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  updateDismiss: vi.fn(async () => ({ kind: "dismissed", version: "0.2.1" })),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { UpdateBanner } from "../src/sidebar/UpdateBanner";
import { useUpdate } from "../src/state/update";

const available: UpdateState = { kind: "available", version: "0.2.1", notes: "Faster canvas", date: null };

function show(state: UpdateState) {
  useUpdate.setState({ state });
  const onInstall = vi.fn();
  render(<UpdateBanner onInstall={onInstall} />);
  return onInstall;
}

beforeEach(() => {
  vi.clearAllMocks();
  useUpdate.setState(useUpdate.getInitialState(), true);
});

describe("update banner", () => {
  it("offers an available update with its notes", async () => {
    const onInstall = show(available);
    const banner = screen.getByRole("status", { name: "Update" });
    expect(banner).toHaveTextContent("Sonata 0.2.1 is available");
    await userEvent.click(screen.getByRole("button", { name: "What's new" }));
    expect(screen.getByText("Faster canvas")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Install and restart" }));
    expect(onInstall).toHaveBeenCalledOnce();
  });

  it("Later dismisses the update", async () => {
    show(available);
    await userEvent.click(screen.getByRole("button", { name: "Later" }));
    expect(ipc.updateDismiss).toHaveBeenCalledOnce();
  });

  it("shows download progress", () => {
    show({ kind: "downloading", version: "0.2.1", received: 50, total: 200 });
    expect(screen.getByRole("status", { name: "Update" })).toHaveTextContent("Downloading Sonata 0.2.1… 25%");
    expect(screen.queryByRole("button", { name: "Install and restart" })).toBeNull();
  });

  it("asks to restart when the update is ready", async () => {
    const onInstall = show({ kind: "readyToInstall", version: "0.2.1" });
    await userEvent.click(screen.getByRole("button", { name: "Restart to update" }));
    expect(onInstall).toHaveBeenCalledOnce();
  });

  it("shows why an update failed and lets the user retry", async () => {
    const onInstall = show({ kind: "failed", version: "0.2.1", reason: "signature check failed" });
    expect(screen.getByRole("status", { name: "Update" })).toHaveTextContent("signature check failed");
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(onInstall).toHaveBeenCalledOnce();
  });

  it("shows why an update is blocked", () => {
    show({ kind: "blocked", version: "0.2.1", reason: "Move Sonata to Applications to update" });
    expect(screen.getByRole("status", { name: "Update" })).toHaveTextContent("Move Sonata to Applications to update");
  });

  it.each<UpdateState>([{ kind: "idle" }, { kind: "dismissed", version: "0.2.1" }])("shows nothing when %o", (state) => {
    show(state);
    expect(screen.queryByRole("status", { name: "Update" })).toBeNull();
  });
});
