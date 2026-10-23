export interface LatestJsonInput {
  version: string;
  notes: string;
  sig: string;
  url: string;
  date: Date;
}

export interface LatestJson {
  version: string;
  notes: string;
  pub_date: string;
  platforms: Record<"darwin-aarch64" | "darwin-x86_64", { signature: string; url: string }>;
}

export function buildLatestJson(input: LatestJsonInput): LatestJson;
