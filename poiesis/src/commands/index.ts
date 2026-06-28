// commands/index.ts
// Composite API client — merges domain-specific API modules (compile, project, annotation)
// with the WebSocket methods (watch, debug) into a single ApiClient class.
// Also exports Tauri backend command wrappers.

import { createCompileApi } from "./compile-api";
import { createProjectApi } from "./project-api";
import { createAnnotationApi } from "./annotation-api";
import { invoke } from "@tauri-apps/api/core";

// ── Tauri backend commands ─────────────────────────

export interface CompileResult {
  success: boolean;
  output: string;
  errors: string[];
}

export async function compileProject(projectPath: string, target: string): Promise<CompileResult> {
  return invoke<CompileResult>("compile_project", { projectPath, target });
}

export async function projectExport(projectPath: string, outputDir: string): Promise<string> {
  return invoke<string>("project_export", { projectPath, outputDir });
}

export async function fileWatch(path: string): Promise<string> {
  return invoke<string>("file_watch", { path });
}

// ── Composite API Client ────────────────────────────

class ApiClient {
  compile = createCompileApi(this.post.bind(this), fetch.bind(globalThis));
  project = createProjectApi(this.post.bind(this), fetch.bind(globalThis));
  annotation = createAnnotationApi(this.post.bind(this), fetch.bind(globalThis));

  // ── Generic POST helper ──────────────────────────

  async post<T = Record<string, unknown>>(
    url: string,
    options?: { body?: BodyInit | null },
  ): Promise<T> {
    const res = await fetch(url, {
      method: "POST",
      headers:
        options?.body != null ? { "Content-Type": "application/json" } : undefined,
      body: options?.body ?? undefined,
    });
    return res.json();
  }

  // ── WebSocket: Watch ────────────────────────────

  private ws: WebSocket | null = null;
  private reconnectTimer: number | null = null;
  private onCompiled: ((data: unknown) => void) | null = null;
  private onStatusChange: ((status: string, isError: boolean) => void) | null = null;

  connectWatch(
    onCompiled: (data: unknown) => void,
    onStatus: (status: string, isError: boolean) => void,
  ): void {
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

  // ── WebSocket: Debug (Phase 2 Runtime Bridge) ───

  private debugWs: WebSocket | null = null;
  private debugReconnectTimer: number | null = null;
  private onDebugMessage: ((msg: Record<string, unknown>) => void) | null = null;
  private onDebugStatusChange: ((connected: boolean) => void) | null = null;

  connectDebugWs(
    onMessage: (msg: Record<string, unknown>) => void,
    onStatusChange: (connected: boolean) => void,
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
      this.debugReconnectTimer = window.setTimeout(
        () => this._doConnectDebug(),
        3000,
      );
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
