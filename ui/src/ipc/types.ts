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

export type TerminalContent = {
  name: string;
  launchCommand?: string;
  workingDirectory?: string;
  saveHistory: boolean;
};

export type NodeContent = { terminal: TerminalContent } | { note: Record<string, never> };

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
export type NoteView = { text: string; sync: NoteSync; conflictPath?: string };
export type NoteChangedEvent = { nodeId: string; text: string; sync: NoteSync };
export type NoteConflictEvent = { nodeId: string; conflictPath: string };

export type BusyWorkspace = { workspaceId: string; workspaceName: string; count: number };

export function isTerminal(node: CanvasNode): node is CanvasNode & { content: { terminal: TerminalContent } } {
  return "terminal" in node.content;
}

export function isNote(node: CanvasNode): boolean {
  return "note" in node.content;
}

export function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}
