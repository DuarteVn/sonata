// Mirrors specs/001-workspaces-canvas-terminals/contracts/ipc.md. Keep in sync with sonata-core.

export type ErrorCode =
  | "NameEmpty"
  | "NameTooLong"
  | "NameTaken"
  | "DirectoryMissing"
  | "NotADirectory"
  | "WorkspaceNotFound"
  | "NodeNotFound"
  | "WorkspaceNotLoaded"
  | "TerminalBusy"
  | "LinkSelf"
  | "LinkExists"
  | "RoleNameTaken"
  | "ImageTooLarge"
  | "ImageUnsupported"
  | "InvalidAddress"
  | "LaneUnavailable"
  | "LaneBlocked"
  | "LaneConflict"
  | "LaneUnsafePath"
  | "LaneHasChanges"
  | "ConfirmationRequired"
  | "LaneConfirmationNeeded"
  | "DeviceInUse"
  | "DeviceNotRunning"
  | "ToolMissing"
  | "Io"
  | "Internal";

export type AppError = { code: ErrorCode; message: string };

export type Preferences = {
  autosaveIntervalSeconds: number;
  terminalFontSize: number;
  historyMaxRows: number;
  /** Omitted when on (feature 005). */
  autoUpdateCheck?: boolean;
};

export type LoadFailureReason =
  | { kind: "invalidJson"; detail: string }
  | { kind: "wrongType"; found: string }
  | { kind: "newerVersion"; found: number; supported: number }
  | { kind: "migrationFailed"; step: number; detail: string }
  | { kind: "decodeFailed"; detail: string }
  | { kind: "idMismatch"; found: string }
  | { kind: "unreadable"; detail: string };

export type LoadFailure = { folderName: string; path: string; reason: LoadFailureReason };

export type WorkspaceSummary = {
  id: string;
  name: string;
  icon: string;
  color: string;
  workingDirectory: string;
  workingDirectoryMissing: boolean;
  isPinned: boolean;
  isLoaded: boolean;
  busyTerminalCount: number;
};

export type Bootstrap = {
  workspaces: WorkspaceSummary[];
  loadFailures: LoadFailure[];
  activeWorkspaceId: string | null;
  crashGuardTripped: boolean;
  recentWorkspaceIds: string[];
  preferences: Preferences;
  warnings: string[];
};

export type Camera = { x: number; y: number; zoom: number };
export type Frame = { x: number; y: number; width: number; height: number };

export type RoleRef = { name?: string; text: string };

export type TerminalContent = {
  name: string;
  launchCommand?: string;
  workingDirectory?: string;
  saveHistory: boolean;
  accentColor?: string;
  fontSize?: number;
  role?: RoleRef;
  monitorActivity?: boolean;
  orchestrator?: boolean;
  recruitedBy?: string;
  sendInstructions?: boolean;
  lane?: Lane;
};

/** A terminal's own git checkout and branch (feature 008). */
export type Lane = {
  branch: string;
  baseBranch: string;
  mainCheckout: string;
  checkoutPath: string;
  subdirectory?: string;
  createdAt: string;
  landedAt?: string;
};

export type LaneAvailability = { available: boolean; reason?: string; baseBranch?: string };

export type LaneState = "ok" | "landed" | "checkoutMissing" | "branchMissing" | "mainCheckoutMissing" | "busy" | "gitUnavailable";

/** `lane://status` (data-model.md). Counts are absent when the lane cannot be read. */
export type LaneStatus = {
  nodeId: string;
  state: LaneState;
  branch: string;
  baseBranch: string;
  changed?: number;
  ahead?: number;
  behind?: number;
};

export type LaneAction = "land" | "remove";
export type LaneRemoval = { discardChanges: boolean; deleteBranch: boolean };

/** `lane://confirm`: an agent asked for a land or removal; a human must answer. */
export type LaneConfirmEvent = {
  requestId: string;
  nodeId: string;
  action: LaneAction;
  requestedBy: string;
  preview: LanePreview;
  deleteBranch: boolean;
};

/** What a land or remove confirmation shows (contracts/ipc.md). */
export type LanePreview = {
  branch: string;
  baseBranch: string;
  commits: number;
  changedFiles: string[];
  changedTotal: number;
  unmergedCommits: number;
  blockers: string[];
};

export type NoteColor = "yellow" | "blue" | "green" | "pink" | "purple" | "gray";
export type NoteStyle = { color: NoteColor; textSize: number };

export type BrowserContent = { url: string; title?: string };
export type Platform = "android" | "ios";
export type DeviceContent = { platform: Platform; deviceId: string; name?: string; osVersion?: string };
export type NodeContent =
  | { terminal: TerminalContent }
  | { note: Partial<NoteStyle> }
  | { browser: BrowserContent }
  | { device: DeviceContent };

/** Installed tools and virtual devices (feature 007, data-model.md). */
export type Avd = { name: string; displayName: string; apiLevel?: number; osVersion?: string; screen?: [number, number] };
export type ToolReport = {
  android: {
    sdk: string | null;
    looked: string[];
    avdHome: string;
    avds: Avd[];
    helpUrl: string;
    avdHelpUrl: string;
  };
  ios: { developerDir: string | null; helpUrl: string };
};

export type DeviceStateName = "off" | "booting" | "ready" | "stopped" | "error";
/** `device://state` (feature 007). */
export type DeviceStateEvent = {
  nodeId: string;
  state: { kind: DeviceStateName; message?: string };
  startedBySonata: boolean;
  reduced: boolean;
  screen?: [number, number];
};

/** `browser://state` from the shell (feature 004). */
export type BrowserState = {
  nodeId: string;
  url: string;
  title: string;
  loading: boolean;
  error?: string;
  image?: string;
  imageVersion: number;
  live: boolean;
};

export type Link = { id: string; a: string; b: string; createdAt: string };

/** For nullable fields: `undefined` keeps the value, `null` clears it. */
export type TerminalSettings = {
  name?: string;
  launchCommand?: string | null;
  workingDirectory?: string | null;
  accentColor?: string | null;
  fontSize?: number | null;
  role?: RoleRef | null;
  monitorActivity?: boolean;
  orchestrator?: boolean;
  sendInstructions?: boolean | null;
};

export type SettingsUpdate = { node: CanvasNode; appliesOnRestart: boolean };
export type PresetStatus = { id: string; found: boolean | null };
export type Role = { id: string; name: string; text: string; createdAt: string };

export type CanvasNode = {
  id: string;
  frame: Frame;
  z: number;
  createdAt: string;
  modifiedAt: string;
  content: NodeContent;
};

export type Workspace = {
  id: string;
  name: string;
  icon: string;
  color: string;
  workingDirectory: string;
  isPinned: boolean;
  createdAt: string;
  lastOpenedAt: string;
  camera: Camera;
  nodes: CanvasNode[];
  links: Link[];
};

export type RunState = "starting" | "running" | "exited";

export type TerminalStateEvent = {
  nodeId: string;
  run: RunState;
  exitCode?: number;
  busy: boolean;
  warning?: string;
};

export type PreviewEvent = { nodeId: string; lines: string[]; cursor?: [number, number] };

export type NoteSync = "clean" | "dirty" | "conflict" | "fileMissing";
export type NoteView = { text: string; sync: NoteSync; conflictPath?: string; assetBase: string };
export type NoteChangedEvent = { nodeId: string; text: string; sync: NoteSync };
export type NoteConflictEvent = { nodeId: string; conflictPath: string };

export type AskStateName = "queued" | "waiting" | "answered" | "replied" | "timedOut" | "failed" | "cancelled";
export type AskEvent = { askId: string; from: string; to: string; state: AskStateName };
export type NotifyEvent = { nodeId: string; title: string; message: string };
export type NodesChangedEvent = { workspaceId: string; nodes: CanvasNode[]; links: Link[] };
export type ExchangeEntry = {
  at: string;
  direction: "in" | "out";
  other: string;
  firstLine: string;
  outcome: AskStateName;
  resultChars: number;
};

export type BusyWorkspace = { workspaceId: string; workspaceName: string; count: number };

export function isTerminal(node: CanvasNode): node is CanvasNode & { content: { terminal: TerminalContent } } {
  return "terminal" in node.content;
}

export function isNote(node: CanvasNode): node is CanvasNode & { content: { note: Partial<NoteStyle> } } {
  return "note" in node.content;
}

export function isBrowser(node: CanvasNode): node is CanvasNode & { content: { browser: BrowserContent } } {
  return "browser" in node.content;
}

export function isDevice(node: CanvasNode): node is CanvasNode & { content: { device: DeviceContent } } {
  return "device" in node.content;
}

export function noteStyle(node: CanvasNode): NoteStyle {
  const n = "note" in node.content ? node.content.note : {};
  return { color: n.color ?? "yellow", textSize: n.textSize ?? 3 };
}

export function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

/** What the update banner shows (feature 005, contracts/ipc.md). */
export type UpdateState =
  | { kind: "idle" }
  | { kind: "available"; version: string; notes: string; date: string | null }
  | { kind: "dismissed"; version: string }
  | { kind: "downloading"; version: string; received: number; total: number | null }
  | { kind: "readyToInstall"; version: string }
  | { kind: "installing"; version: string }
  | { kind: "failed"; version: string; reason: string }
  | { kind: "blocked"; version: string; reason: string };

// Usage meter (feature 006, contracts/ipc.md). Empty optional fields are omitted by core.
export type WindowKind = "short" | "weekly" | "other";

export type UsageWindow = {
  kind: WindowKind;
  label: string;
  usedPercent: number;
  resetsAt?: string | null;
};

export type ReadingStatus = "ok" | "noPlanLimits" | "notLoggedIn" | "notInstalled" | "unavailable" | "off";

export type Reading = {
  providerId: string;
  status: ReadingStatus;
  plan?: string | null;
  account?: string | null;
  accountEmail?: string | null;
  windows?: UsageWindow[];
  readAt: string;
  message?: string | null;
};

export type ProviderState = {
  id: string;
  name: string;
  kind: "builtIn" | "custom";
  color: string;
  enabled: boolean;
  detection: "found" | "notFound" | "unknown";
  commandPath: string | null;
  consent: "unasked" | "granted" | "denied";
  command: string[];
  definitionError: string | null;
  reading: Reading | null;
  stale: boolean;
  lastError: string | null;
  refreshing: boolean;
};

export type UsageSummary = {
  state: "off" | "empty" | "normal" | "warning" | "full";
  percent: number | null;
  providerId: string | null;
  windowKind: WindowKind | null;
  resetsAt: string | null;
};

export type UsageState = {
  enabled: boolean;
  warnings: boolean;
  summary: UsageSummary;
  providers: ProviderState[];
  providersFolder: string;
};

export type UsageWarning = {
  providerId: string;
  providerName: string;
  windowLabel: string;
  level: 80 | 100;
  resetsAt: string | null;
};

export type ConsentRequest = {
  providerId: string;
  providerName: string;
  kind: "builtIn" | "custom";
  command: string[];
  explanation: string;
};

export type RefreshReason = "panelOpen" | "manual";
