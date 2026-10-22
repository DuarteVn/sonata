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
  | "Io"
  | "Internal";

export type AppError = { code: ErrorCode; message: string };

export type Preferences = {
  autosaveIntervalSeconds: number;
  terminalFontSize: number;
  historyMaxRows: number;
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
};

export type NoteColor = "yellow" | "blue" | "green" | "pink" | "purple" | "gray";
export type NoteStyle = { color: NoteColor; textSize: number };

export type NodeContent = { terminal: TerminalContent } | { note: Partial<NoteStyle> };

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

export function noteStyle(node: CanvasNode): NoteStyle {
  const n = "note" in node.content ? node.content.note : {};
  return { color: n.color ?? "yellow", textSize: n.textSize ?? 3 };
}

export function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}
