import type { ProviderState, Reading, UsageState } from "../src/ipc/types";

export const NOW = new Date("2026-10-23T12:00:00Z");

export function reading(over: Partial<Reading> = {}): Reading {
  return {
    providerId: "claude",
    status: "ok",
    plan: "Team",
    account: "Example Org",
    accountEmail: "user@example.com",
    windows: [
      { kind: "short", label: "5-hour", usedPercent: 76, resetsAt: "2026-10-23T14:10:00Z" },
      { kind: "weekly", label: "Weekly", usedPercent: 13, resetsAt: "2026-10-30T07:00:00Z" },
    ],
    readAt: "2026-10-23T11:58:00Z",
    ...over,
  };
}

export function provider(over: Partial<ProviderState> = {}): ProviderState {
  return {
    id: "claude",
    name: "Claude Code",
    kind: "builtIn",
    color: "#d97757",
    enabled: true,
    detection: "found",
    commandPath: "/opt/homebrew/bin/claude",
    consent: "granted",
    command: ["claude", "-p", "."],
    definitionError: null,
    reading: reading(),
    stale: false,
    lastError: null,
    refreshing: false,
    ...over,
  };
}

export function usageState(providers: ProviderState[], over: Partial<UsageState> = {}): UsageState {
  return {
    enabled: true,
    warnings: true,
    summary: { state: "normal", percent: 76, providerId: "claude", windowKind: "short", resetsAt: "2026-10-23T14:10:00Z" },
    providers,
    providersFolder: "/Users/me/.sonata/usage-providers",
    ...over,
  };
}
