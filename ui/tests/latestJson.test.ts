import { describe, expect, it } from "vitest";
import { buildLatestJson } from "../../scripts/release/latest-json.mjs";

const url = "https://github.com/DuarteVn/sonata/releases/download/v0.2.0/Sonata.app.tar.gz";
const base = { version: "v0.2.0", notes: "Fixes", sig: "  c2lnbmF0dXJl\n", url, date: new Date("2026-10-30T12:00:00.000Z") };

describe("buildLatestJson", () => {
  it("builds the manifest the updater reads", () => {
    expect(buildLatestJson(base)).toEqual({
      version: "0.2.0",
      notes: "Fixes",
      pub_date: "2026-10-30T12:00:00Z",
      platforms: {
        "darwin-aarch64": { signature: "c2lnbmF0dXJl", url },
        "darwin-x86_64": { signature: "c2lnbmF0dXJl", url },
      },
    });
  });

  it("keeps a version without a leading v", () => {
    expect(buildLatestJson({ ...base, version: "1.2.3" }).version).toBe("1.2.3");
  });

  it("refuses an empty signature", () => {
    expect(() => buildLatestJson({ ...base, sig: " \n" })).toThrow(/signature/);
  });

  it("refuses a URL that is not https", () => {
    expect(() => buildLatestJson({ ...base, url: "http://example.com/a.tar.gz" })).toThrow(/https/);
  });
});
