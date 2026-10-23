import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { QuitDialog } from "../src/dialogs/QuitDialog";

describe("quit confirmation (007 FR-008)", () => {
  it("lists running devices that quitting stops", () => {
    render(
      <QuitDialog
        busy={[
          { workspaceId: "a", workspaceName: "App", count: 0, devices: ["Pixel 8"] },
          { workspaceId: "b", workspaceName: "Api", count: 2 },
        ]}
        onQuit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByText(/2 programs are still running/)).toBeInTheDocument();
    expect(screen.getByText(/1 device is still running/)).toBeInTheDocument();
    expect(screen.getByText(/Pixel 8/)).toBeInTheDocument();
  });

  it("talks only about devices when no program runs", () => {
    render(<QuitDialog busy={[{ workspaceId: "a", workspaceName: "App", count: 0, devices: ["Pixel 8", "Tablet"] }]} onQuit={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.queryByText(/programs? (is|are) still running/)).toBeNull();
    expect(screen.getByText(/2 devices are still running/)).toBeInTheDocument();
  });
});
