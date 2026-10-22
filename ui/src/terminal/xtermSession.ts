import { Channel } from "@tauri-apps/api/core";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { WebglAddon } from "@xterm/addon-webgl";
import { Terminal, type ITheme } from "@xterm/xterm";
import { ipc } from "../ipc/commands";
import type { TerminalStateEvent } from "../ipc/types";

export const theme: ITheme = {
  background: "#101115",
  foreground: "#e7e8ec",
  cursor: "#e7e8ec",
  selectionBackground: "#3a4160",
};

export type SessionOptions = {
  baseFontSize: number;
  zoom: number;
  useWebgl: boolean;
  onState?: (state: TerminalStateEvent) => void;
  onWebglLost?: () => void;
};

// One live xterm.js view of a terminal that runs in the core. Creating it attaches (the core
// sends a replay, then live bytes); disposing it detaches without stopping the process.
export class XtermSession {
  private readonly term: Terminal;
  private readonly fit = new FitAddon();
  private webgl: WebglAddon | undefined;
  private cols = 80;
  private rows = 24;
  private disposed = false;

  constructor(
    private readonly nodeId: string,
    private readonly el: HTMLElement,
    private readonly opts: SessionOptions,
  ) {
    this.zoom = opts.zoom;
    this.term = new Terminal({
      fontSize: opts.baseFontSize * opts.zoom,
      fontFamily: '"SF Mono", Menlo, Monaco, monospace',
      scrollback: 10000,
      cursorBlink: true,
      allowProposedApi: true,
      macOptionIsMeta: true,
      theme,
    });
    this.term.loadAddon(this.fit);
    this.term.loadAddon(new Unicode11Addon());
    this.term.unicode.activeVersion = "11";
  }

  async start(): Promise<void> {
    this.term.open(this.el);
    if (this.opts.useWebgl) this.enableWebgl();
    const dims = this.fit.proposeDimensions();
    if (dims && dims.cols > 0 && dims.rows > 0) {
      this.cols = dims.cols;
      this.rows = dims.rows;
    }
    this.term.resize(this.cols, this.rows);
    this.term.onData((data) => void ipc.terminalWrite(this.nodeId, data));
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = (data) => {
      if (!this.disposed) this.term.write(new Uint8Array(data));
    };
    const state = await ipc.terminalAttach(this.nodeId, channel, this.cols, this.rows);
    if (state) this.opts.onState?.(state);
  }

  enableWebgl() {
    if (this.webgl || this.disposed) return;
    try {
      const addon = new WebglAddon();
      addon.onContextLoss(() => {
        this.disableWebgl();
        this.opts.onWebglLost?.();
      });
      this.term.loadAddon(addon);
      this.webgl = addon;
    } catch {
      this.webgl = undefined;
    }
  }

  disableWebgl() {
    this.webgl?.dispose();
    this.webgl = undefined;
  }

  get usesWebgl() {
    return this.webgl !== undefined;
  }

  private zoom = 1;

  /** Scales the font with the canvas zoom. The grid (cols × rows) stays the same. */
  setZoom(zoom: number) {
    this.zoom = zoom;
    this.term.options.fontSize = this.opts.baseFontSize * zoom;
    this.term.resize(this.cols, this.rows);
  }

  /** A new per-terminal font size changes the grid, so refit and tell the core. */
  setBaseFontSize(size: number) {
    if (size === this.opts.baseFontSize) return;
    this.opts.baseFontSize = size;
    this.term.options.fontSize = size * this.zoom;
    this.refit();
  }

  /** Recomputes the grid after the node was resized and tells the core. */
  refit() {
    const dims = this.fit.proposeDimensions();
    if (!dims || dims.cols <= 0 || dims.rows <= 0) return;
    if (dims.cols === this.cols && dims.rows === this.rows) return;
    this.cols = dims.cols;
    this.rows = dims.rows;
    this.term.resize(this.cols, this.rows);
    void ipc.terminalResize(this.nodeId, this.cols, this.rows);
  }

  focus() {
    this.term.focus();
  }

  blur() {
    this.term.blur();
  }

  dispose() {
    if (this.disposed) return;
    this.disposed = true;
    void ipc.terminalDetach(this.nodeId);
    this.disableWebgl();
    this.term.dispose();
  }
}
