/** HTTP + WebSocket 通信封装。*/

import type { CompileResponse, FsEntry, ProjectInfo } from "./types";

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

  // M6: .omni.json helpers
  async exportOmniJson(source: string, filepath = "<input>", target = "react"): Promise<{ success: boolean; document: Record<string, unknown> | null; error?: string }> {
    return this.post("/api/omni-json/export", {
      body: JSON.stringify({ source, filepath, target }),
    });
  }

  async loadOmniJson(filePath: string): Promise<{ success: boolean; document: Record<string, unknown> | null; error?: string }> {
    return this.post("/api/omni-json/load", {
      body: JSON.stringify({ filePath }),
    });
  }

  async saveOmniJson(filePath: string, document: Record<string, unknown>): Promise<{ ok: boolean; path?: string; error?: string }> {
    return this.post("/api/omni-json/save", {
      body: JSON.stringify({ filePath, document }),
    });
  }

  async detectHandWritten(source: string): Promise<{ isHandWritten: boolean; warnings: string[] }> {
    return this.post("/api/omni-json/detect-handwritten", {
      body: JSON.stringify({ source }),
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
      this.onStatusChange?.("🔌 重连中...", true);
      this._stopHeartbeat();
      this.reconnectTimer = window.setTimeout(() => this._doConnect(), 3000);
    };
  }

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
}

export const api = new ApiClient();
