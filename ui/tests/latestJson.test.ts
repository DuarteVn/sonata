import { describe, expect, it } from "vitest";
import { buildLatestJson, parseArgs } from "../../scripts/release/latest-json.mjs";

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

  const appImage = "https://github.com/DuarteVn/sonata/releases/download/v0.2.0/Sonata_0.2.0_amd64.AppImage";

  it("adds the Linux AppImage when given and keeps the Mac entries", () => {
    const manifest = buildLatestJson({ ...base, linux: { sig: "bGludXg=\n", url: appImage } });
    expect(manifest.platforms).toEqual({
      "darwin-aarch64": { signature: "c2lnbmF0dXJl", url },
      "darwin-x86_64": { signature: "c2lnbmF0dXJl", url },
      "linux-x86_64": { signature: "bGludXg=", url: appImage },
    });
  });

  it("checks the Linux entry like the Mac one", () => {
    expect(() => buildLatestJson({ ...base, linux: { sig: "", url: appImage } })).toThrow(/signature/);
    expect(() => buildLatestJson({ ...base, linux: { sig: "x", url: "http://a/b.AppImage" } })).toThrow(/https/);
  });
});

describe("parseArgs", () => {
  const mac = ["--version", "v0.2.0", "--notes-file", "n.md", "--sig-file", "a.sig", "--url", url];

  it("accepts the Mac flags alone", () => {
    expect(parseArgs(mac)["linux-url"]).toBeUndefined();
  });

  it("needs both Linux flags or neither", () => {
    expect(parseArgs([...mac, "--linux-sig-file", "l.sig", "--linux-url", "https://x/y"])["linux-url"]).toBe("https://x/y");
    expect(() => parseArgs([...mac, "--linux-sig-file", "l.sig"])).toThrow(/--linux-url/);
    expect(() => parseArgs([...mac, "--linux-url", "https://x/y"])).toThrow(/--linux-sig-file/);
  });
});
