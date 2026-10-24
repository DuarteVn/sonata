import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ConsentRequest } from "../src/ipc/types";

const ipc = vi.hoisted(() => ({
  usageAnswerConsent: vi.fn(async () => undefined as unknown),
}));
vi.mock("../src/ipc/commands", () => ({ ipc }));

import { UsageConsentDialog } from "../src/dialogs/UsageConsentDialog";

const request: ConsentRequest = {
  providerId: "claude",
  providerName: "Claude Code",
  kind: "builtIn",
  command: ["/opt/homebrew/bin/claude", "-p", ".", "--tools", "", "--system-prompt", "Reply with one dot."],
  explanation: "Sonata runs a tiny request.",
};

beforeEach(() => vi.clearAllMocks());

describe("usage consent dialog", () => {
  it("shows the exact command and the explanation", () => {
    render(<UsageConsentDialog request={request} onDone={() => {}} />);
    const dialog = screen.getByRole("dialog", { name: "Read Claude Code usage?" });
    expect(dialog).toHaveTextContent("/opt/homebrew/bin/claude -p . --tools '' --system-prompt 'Reply with one dot.'");
    expect(dialog).toHaveTextContent("Sonata runs a tiny request.");
  });

  it("Allow sends the same command back", async () => {
    const onDone = vi.fn();
    render(<UsageConsentDialog request={request} onDone={onDone} />);
    await userEvent.click(screen.getByRole("button", { name: "Allow" }));
    expect(ipc.usageAnswerConsent).toHaveBeenCalledWith("claude", true, request.command);
    expect(onDone).toHaveBeenCalled();
  });

  it("Don't allow records a no", async () => {
    render(<UsageConsentDialog request={request} onDone={() => {}} />);
    await userEvent.click(screen.getByRole("button", { name: "Don't allow" }));
    expect(ipc.usageAnswerConsent).toHaveBeenCalledWith("claude", false, request.command);
  });
});
