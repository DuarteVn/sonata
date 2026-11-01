import { describe, expect, it } from "vitest";

// Feature 012 (SC-007): text that names macOS keys or apps must sit behind an `isMac` choice,
// so Windows never shows Cmd, ⌘, ⌃, Finder, or macOS.
const MAC_ONLY = /⌘|⌃|\bCmd\b|Finder|macOS/;
const SOURCES = import.meta.glob("../src/**/*.{ts,tsx}", { query: "?raw", import: "default", eager: true }) as Record<
  string,
  string
>;

describe("text shown on Windows", () => {
  it("names macOS keys and apps only behind an isMac choice", () => {
    expect(Object.keys(SOURCES).length).toBeGreaterThan(20);
    const offenders = Object.entries(SOURCES)
      .filter(([file]) => !file.endsWith("platform.ts"))
      .flatMap(([file, text]) =>
        text.split("\n").flatMap((line, i) => {
          const code = line.trim();
          const comment = code.startsWith("//") || code.startsWith("*") || code.startsWith("/*");
          return !comment && MAC_ONLY.test(code) && !code.includes("isMac") ? [`${file}:${i + 1}: ${code}`] : [];
        }),
      );
    expect(offenders).toEqual([]);
  });
});
