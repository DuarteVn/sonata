export interface LatestJsonInput {
  version: string;
  notes: string;
  sig: string;
  url: string;
  date: Date;
  /** The Linux x86_64 AppImage and its signature (feature 011). */
  linux?: { sig: string; url: string };
  /** The Windows x86_64 installer and its signature (feature 012). */
  windows?: { sig: string; url: string };
}

export type Platform = "darwin-aarch64" | "darwin-x86_64" | "linux-x86_64" | "windows-x86_64";

export interface LatestJson {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Partial<Record<Platform, { signature: string; url: string }>>;
}

export function buildLatestJson(input: LatestJsonInput): LatestJson;

export function parseArgs(argv: string[]): Record<string, string>;
