/** HTTP + WebSocket 通信封装。*/

import type { CompileResponse, FsEntry, ProjectInfo } from "./types";
import type { AnnotatedLoadResponse, AnnotatedSaveResponse, EditorTomlState, AnnotationFile } from "../core/types";

class ApiClient {
  private ws: WebSocket | null = null;
  private reconnectTimer: number | null = null;
  private onCompiled: ((data: CompileResponse) => void) | null = null;
  private onStatusChange: ((status: string, isError: boolean) => void) | null = null;

  // ── HTTP ───────────────────────────────────

  async compile(source: string, target = "react", previousFiles?: Record<string, string>, debug = false): Promise<CompileResponse> {
    const res = await fetch("/api/compile", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source, target, previousFiles, debug }),
    });
    return res.json();
  }

  async listDir(dirPath = "."): Promise<{ entries: FsEntry[]; error?: string }> {
    const res = await fetch("/api/fs/list", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ dirPath }),
    });
    return res.json();
  }

  async readFile(filePath: string): Promise<{ content: string; error?: string }> {
    const res = await fetch("/api/fs/read", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath }),
    });
    return res.json();
  }

  async writeFile(filePath: string, content: string): Promise<{ ok: boolean; error?: string }> {
    const res = await fetch("/api/fs/write", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ filePath, content }),
    });
    return res.json();
  }

  async getProject(): Promise<ProjectInfo> {
    const res = await fetch("/api/project");
    return res.json();
  }

  /** Generic POST helper for M6+ endpoints. */
  async post<T = Record<string, unknown>>(
    url: string,
    options?: { body?: BodyInit | null }
  ): Promise<T> {
    const res = await fetch(url, {
      method: "POST",
      headers: options?.body != null ? { "Content-Type": "application/json" } : undefined,
      body: options?.body ?? undefined,
    });
    return res.json();
  }

  // ── Custom component definitions ──
  async saveComponentDefinitions(definitions: Record<string, unknown>): Promise<{ ok: boolean; error?: string }> {
    return this.post("/api/component-definitions/save", {
      body: JSON.stringify({ definitions }),
    });
  }

  async loadComponentDefinitions(): Promise<{ definitions: Record<string, unknown>; error?: string }> {
    const res = await fetch("/api/component-definitions/load");
    return res.json();
  }

  async deleteComponentDefinitions(names: string[]): Promise<{ ok: boolean; error?: string }> {
    return this.post("/api/component-definitions/delete", {
      body: JSON.stringify({ names }),
    });
  }

  // ── Run / Vite DevServer (Phase 1 Runtime Bridge) ──

  async startRun(files: Record<string, string>, debug = false): Promise<{ success: boolean; url?: string; port?: number; error?: string }> {
    return this.post("/api/run/start", {
      body: JSON.stringify({ files, debug }),
    });
  }

  async stopRun(): Promise<{ success: boolean }> {
    return this.post("/api/run/stop");
  }

  async getRunStatus(): Promise<{ running: boolean; url: string | null }> {
    const res = await fetch("/api/run/status");
    return res.json();
  }

  async writeRunFiles(files: Record<string, string>): Promise<{ success: boolean; count?: number; error?: string }> {
    return this.post("/api/run/write-files", {
      body: JSON.stringify({ files }),
    });
  }

  // ── WebSocket ─────────────────────────────

  connectWatch(onCompiled: (data: CompileResponse) => void, onStatus: (status: string, isError: boolean) => void): void {
    this.onCompiled = onCompiled;
    this.onStatusChange = onStatus;
    this._doConnect();
  }

  private _doConnect(): void {
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    this.ws = new WebSocket(`${protocol}//${location.host}/api/watch`);

    this.ws.onopen = () => {
      this.everConnected = true;
      this.onStatusChange?.("就绪", false);
      this._startHeartbeat();
    };

    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === "compiled") {
        this.onCompiled?.(data);
      } else if (data.type === "error") {
        this.onStatusChange?.(data.message, true);
      }
    };

    this.ws.onclose = () => {
      if (this.everConnected) {
        this.onStatusChange?.("🔌 重连中...", true);
      }
      this._stopHeartbeat();
      this.reconnectTimer = window.setTimeout(() => this._doConnect(), 3000);
    };
  }

  private everConnected = false;
  private _heartbeatInterval: number | null = null;

  private _startHeartbeat(): void {
    this._heartbeatInterval = window.setInterval(() => {
      if (this.ws?.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ type: "ping" }));
      }
    }, 5000);
  }

  private _stopHeartbeat(): void {
    if (this._heartbeatInterval !== null) {
      clearInterval(this._heartbeatInterval);
      this._heartbeatInterval = null;
    }
  }

  disconnect(): void {
    if (this.reconnectTimer !== null) clearTimeout(this.reconnectTimer);
    this._stopHeartbeat();
    this.ws?.close();
  }

  // ── Debug WebSocket (Phase 2 Runtime Bridge) ──────────

  private debugWs: WebSocket | null = null;
  private debugReconnectTimer: number | null = null;
  private onDebugMessage: ((msg: Record<string, unknown>) => void) | null = null;
  private onDebugStatusChange: ((connected: boolean) => void) | null = null;

  connectDebugWs(
    onMessage: (msg: Record<string, unknown>) => void,
    onStatusChange: (connected: boolean) => void
  ): void {
    this.onDebugMessage = onMessage;
    this.onDebugStatusChange = onStatusChange;
    this._doConnectDebug();
  }

  private _doConnectDebug(): void {
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    this.debugWs = new WebSocket(`${protocol}//${location.host}/api/debug/ws`);

    this.debugWs.onopen = () => {
      this.onDebugStatusChange?.(true);
    };

    this.debugWs.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data as string);
        this.onDebugMessage?.(data);
      } catch {
        // Ignore malformed debug messages
      }
    };

    this.debugWs.onclose = () => {
      this.onDebugStatusChange?.(false);
      this.debugReconnectTimer = window.setTimeout(() => this._doConnectDebug(), 3000);
    };
  }

  sendDebugCommand(command: Record<string, unknown>): void {
    if (this.debugWs?.readyState === WebSocket.OPEN) {
      this.debugWs.send(JSON.stringify(command));
    }
  }

  disconnectDebug(): void {
    if (this.debugReconnectTimer !== null) clearTimeout(this.debugReconnectTimer);
    this.debugWs?.close();
    this.debugWs = null;
    this.onDebugMessage = null;
    this.onDebugStatusChange = null;
  }

  // ── Annotation API ──
  async annotatedSave(filePath: string, annotations: AnnotationFile, source: string): Promise<AnnotatedSaveResponse> {
    return this.post("/api/annotated-save", {
      body: JSON.stringify({ filePath, annotations, source }),
    });
  }

  async annotatedLoad(filePath: string): Promise<AnnotatedLoadResponse> {
    return this.post("/api/annotated-load", {
      body: JSON.stringify({ filePath }),
    });
  }

  async saveEditorToml(state: EditorTomlState): Promise<{ ok: boolean; error?: string }> {
    return this.post("/api/editor-toml/save", {
      body: JSON.stringify({ state }),
    });
  }

  async loadEditorToml(): Promise<{ state: EditorTomlState | null; error?: string }> {
    const res = await fetch("/api/editor-toml/load");
    return res.json();
  }

  // ── Welcome screen API ─────────────────────

  async listDirectory(path: string): Promise<{ ok: boolean; current?: string; entries?: Array<{ name: string; isDir: boolean }>; error?: string }> {
    const res = await fetch("/api/fs/list-directory", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path }),
    });
    return res.json();
  }

  async setProjectDir(dir: string): Promise<{ ok: boolean; dir?: string; error?: string }> {
    const res = await fetch("/api/project/set-dir", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ dir }),
    });
    return res.json();
  }

  async createProject(dir: string, name: string): Promise<{ ok: boolean; dir?: string; error?: string }> {
    const res = await fetch("/api/project/create", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ dir, name }),
    });
    return res.json();
  }
}

export const api = new ApiClient();
