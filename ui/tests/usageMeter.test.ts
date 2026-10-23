import { describe, expect, it } from "vitest";
import { meterLevel, percentText, resetText, shellQuote, timeAgo } from "../src/usage/meter";
import { NOW } from "./usageFixtures";

describe("usage meter helpers", () => {
  it("formats percentages", () => {
    expect(percentText(76)).toBe("76%");
    expect(percentText(50.12)).toBe("50%");
    expect(percentText(0.3)).toBe("<1%");
    expect(percentText(0)).toBe("0%");
    expect(percentText(130)).toBe("100%");
  });

  it("describes reset times relative to now", () => {
    expect(resetText("2026-10-23T14:10:00Z", NOW)).toBe("resets in 2 h 10 min");
    expect(resetText("2026-10-23T12:25:00Z", NOW)).toBe("resets in 25 min");
    expect(resetText("2026-10-30T07:00:00Z", NOW)).toBe("resets in 6 d 19 h");
    expect(resetText("2026-10-23T11:00:00Z", NOW)).toBe("resetting now");
    expect(resetText(null, NOW)).toBe("");
  });

  it("describes reading age", () => {
    expect(timeAgo("2026-10-23T11:59:40Z", NOW)).toBe("just now");
    expect(timeAgo("2026-10-23T11:57:00Z", NOW)).toBe("3 min ago");
    expect(timeAgo("2026-10-23T09:00:00Z", NOW)).toBe("3 h ago");
    expect(timeAgo("2026-10-20T12:00:00Z", NOW)).toBe("3 d ago");
  });

  it("maps percentages to meter levels", () => {
    expect(meterLevel(79.9)).toBe("normal");
    expect(meterLevel(80)).toBe("warning");
    expect(meterLevel(100)).toBe("full");
  });

  it("quotes commands for display", () => {
    expect(shellQuote(["/bin/claude", "-p", ".", "--tools", "", "Reply with one dot."])).toBe(
      "/bin/claude -p . --tools '' 'Reply with one dot.'",
    );
  });
});
