/**
 * OmniLang Runtime Bridge — injected into compiled React apps for debug mode.
 *
 * Provides Proxy-wrapped React hooks that track signal/state changes and
 * relay them back to the OmniLang Editor via WebSocket for live debugging.
 *
 * Usage (auto-injected by the compiler when debug_mode=true):
 *   import { useTrackedSignal, useTrackedComputed, createTrackedStore } from "./omni-runtime";
 *
 * Protocol (WebSocket JSON):
 *   App → Editor: state-change, execution-trace, runtime-ready, runtime-error
 *   Editor → App: set-breakpoint, remove-breakpoint, step, resume, get-state
 */

import { useState, useMemo, useRef, useEffect, useCallback } from "react";
import type { StateCreator } from "zustand";
import { create } from "zustand";

// ── Types ────────────────────────────────────────────────

interface SignalMeta {
  signalId: string;
  name: string;
  type: string;
}

interface StateChangeMessage {
  type: "state-change";
  signalId: string;
  name: string;
  value: unknown;
  timestamp: number;
}

interface ExecutionTraceMessage {
  type: "execution-trace";
  nodeId: string;
  edgeId?: string;
  timestamp: number;
}

interface RuntimeMessage {
  type: "runtime-ready" | "runtime-error";
  message?: string;
}

type OutgoingMessage = StateChangeMessage | ExecutionTraceMessage | RuntimeMessage | {
  type: "paused" | "resumed";
  signalId?: string;
  value?: unknown;
  timestamp: number;
};

interface IncomingMessage {
  type: "set-breakpoint" | "remove-breakpoint" | "step" | "resume" | "get-state";
  signalId?: string;
}

// ── WebSocket Bridge (singleton) ─────────────────────────

class OmniRuntimeBridge {
  private ws: WebSocket | null = null;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private messageQueue: OutgoingMessage[] = [];
  private listeners: Map<string, Set<(msg: IncomingMessage) => void>> = new Map();
  private _connected = false;
  private wsUrl: string;

  /** Active breakpoints — set of signal IDs that should pause execution. */
  breakpoints: Set<string> = new Set();

  /** Whether the runtime is currently paused on a breakpoint. */
  private _paused = false;

  /** Deferred state updates waiting to be flushed by step/resume. */
  private _pendingQueue: Array<{
    signalId: string;
    value: unknown;
    setValue: (v: unknown) => void;
  }> = [];

  /** When true, bypass breakpoint checks (prevent re-entrant queueing during flush). */
  private _flushing = false;

  constructor() {
    // Read debug WebSocket URL from Vite env or global
    this.wsUrl =
      (typeof window !== "undefined" &&
        (window as Record<string, unknown>).__OMNI_DEBUG_WS_URL__ as string) ||
      "ws://localhost:8942/api/debug/ws";
    this.connect();
    this._registerDebugHandlers();
  }

  get connected(): boolean {
    return this._connected;
  }

  private connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) return;

    try {
      this.ws = new WebSocket(this.wsUrl);

      this.ws.onopen = () => {
        this._connected = true;
        this.send({ type: "runtime-ready" });
        // Flush queued messages
        for (const msg of this.messageQueue) {
          this.sendRaw(msg);
        }
        this.messageQueue = [];
      };

      this.ws.onmessage = (event) => {
        try {
          const msg: IncomingMessage = JSON.parse(event.data as string);
          const handlers = this.listeners.get(msg.type);
          if (handlers) {
            for (const h of handlers) {
              h(msg);
            }
          }
        } catch {
          // Ignore malformed messages
        }
      };

      this.ws.onclose = () => {
        this._connected = false;
        this.scheduleReconnect();
      };

      this.ws.onerror = () => {
        // onclose will fire after this
      };
    } catch {
      this.scheduleReconnect();
    }
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) return;
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, 2000);
  }

  send(msg: OutgoingMessage): void {
    if (this._connected && this.ws?.readyState === WebSocket.OPEN) {
      this.sendRaw(msg);
    } else {
      // Queue messages when disconnected (max 100)
      if (this.messageQueue.length < 100) {
        this.messageQueue.push(msg);
      }
    }
  }

  private sendRaw(msg: OutgoingMessage): void {
    try {
      this.ws?.send(JSON.stringify(msg));
    } catch {
      // Silently drop — debug bridge is best-effort
    }
  }

  on(msgType: IncomingMessage["type"], handler: (msg: IncomingMessage) => void): () => void {
    if (!this.listeners.has(msgType)) {
      this.listeners.set(msgType, new Set());
    }
    this.listeners.get(msgType)!.add(handler);
    return () => {
      this.listeners.get(msgType)?.delete(handler);
    };
  }

  /** Send execution trace (node/edge activation). */
  trace(nodeId: string, edgeId?: string): void {
    this.send({
      type: "execution-trace",
      nodeId,
      edgeId,
      timestamp: Date.now(),
    });
  }

  // ── Breakpoint / Pause State Machine ───────────────────

  get isPaused(): boolean {
    return this._paused;
  }

  get isFlushing(): boolean {
    return this._flushing;
  }

  hasBreakpoint(signalId: string): boolean {
    return this.breakpoints.has(signalId);
  }

  /** Defer a state update — queue it and pause execution until step/resume. */
  queuePending(signalId: string, value: unknown, setValue: (v: unknown) => void): void {
    // Guard against React StrictMode double-invoke: update in place
    const existing = this._pendingQueue.find((p) => p.signalId === signalId);
    if (existing) {
      existing.value = value;
      return;
    }
    this._pendingQueue.push({ signalId, value, setValue });
    this._paused = true;

    this.send({
      type: "paused",
      signalId,
      value,
      timestamp: Date.now(),
    });
  }

  private _registerDebugHandlers(): void {
    this.on("set-breakpoint", (msg) => {
      if (msg.signalId) {
        this.breakpoints.add(msg.signalId);
      }
    });

    this.on("remove-breakpoint", (msg) => {
      if (msg.signalId) {
        this.breakpoints.delete(msg.signalId);
      }
    });

    this.on("step", () => {
      this._step();
    });

    this.on("resume", () => {
      this._resume();
    });
  }

  private _resume(): void {
    if (this._pendingQueue.length === 0) return;

    this._flushing = true;
    while (this._pendingQueue.length > 0) {
      const pending = this._pendingQueue.shift()!;
      pending.setValue(pending.value);
    }
    this._paused = false;
    this._flushing = false;

    this.send({
      type: "resumed",
      timestamp: Date.now(),
    });
  }

  private _step(): void {
    if (this._pendingQueue.length === 0) return;

    this._flushing = true;
    const pending = this._pendingQueue.shift()!;
    pending.setValue(pending.value);
    this._flushing = false;

    if (this._pendingQueue.length === 0) {
      this._paused = false;
      this.send({
        type: "resumed",
        timestamp: Date.now(),
      });
    } else {
      // Still paused — notify editor of next pending signal
      this.send({
        type: "paused",
        signalId: this._pendingQueue[0].signalId,
        value: this._pendingQueue[0].value,
        timestamp: Date.now(),
      });
    }
  }
}

// Global singleton — one per app instance
const bridge = new OmniRuntimeBridge();

// Expose for debugging in browser console
if (typeof window !== "undefined") {
  (window as Record<string, unknown>).__OMNI_RUNTIME__ = bridge;
}

// ── Tracked Signal (Proxy-wrapped useState) ──────────────

/**
 * Wraps React useState with a Proxy that tracks reads/writes and
 * sends state changes to the Editor via WebSocket.
 */
export function useTrackedSignal<T>(
  initialValue: T,
  signalId: string,
  name: string,
  type: string
): [T, (value: T | ((prev: T) => T)) => void] {
  const [value, setValue] = useState<T>(initialValue);
  const metaRef = useRef<SignalMeta>({ signalId, name, type });

  // Stable reference to the raw setter — used by deferred flush (bridge.queuePending)
  const rawSetValueRef = useRef(setValue);
  rawSetValueRef.current = setValue;

  // Report initial value on mount
  useEffect(() => {
    bridge.send({
      type: "state-change",
      signalId,
      name,
      value: initialValue,
      timestamp: Date.now(),
    });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const trackedSetValue = useCallback(
    (newValue: T | ((prev: T) => T)) => {
      setValue((prev) => {
        const resolved =
          typeof newValue === "function"
            ? (newValue as (prev: T) => T)(prev)
            : newValue;

        // Send state change to Editor (always — editor sees value even if deferred)
        bridge.send({
          type: "state-change",
          signalId,
          name: metaRef.current.name,
          value: resolved,
          timestamp: Date.now(),
        });

        // Check breakpoint: defer if hit and not in flush mode
        if (!bridge.isFlushing && bridge.hasBreakpoint(signalId)) {
          bridge.queuePending(signalId, resolved, rawSetValueRef.current);
          return prev; // Return unchanged — defer the React state update
        }

        return resolved;
      });
    },
    [signalId, name]
  );

  return [value, trackedSetValue];
}

// ── Tracked Computed (Proxy-wrapped useMemo) ─────────────

/**
 * Wraps React useMemo with tracking that sends computed value
 * changes to the Editor via WebSocket.
 */
export function useTrackedComputed<T>(
  factory: () => T,
  deps: unknown[],
  computedId: string,
  name: string
): T {
  const value = useMemo(factory, deps); // eslint-disable-line react-hooks/exhaustive-deps

  // Report computed value on change
  useEffect(() => {
    bridge.send({
      type: "state-change",
      signalId: computedId,
      name,
      value,
      timestamp: Date.now(),
    });
  }, [value]); // eslint-disable-line react-hooks/exhaustive-deps

  return value;
}

// ── Tracked Store (Zustand middleware) ────────────────────

/**
 * Zustand middleware that tracks all state changes and relays
 * them to the Editor via WebSocket.
 */
function omniDebugMiddleware<T extends Record<string, unknown>>(
  storeName: string
) {
  return (config: StateCreator<T>): StateCreator<T> => {
    return (set, get, api) => {
      const trackedSet: typeof set = (partial, replace) => {
        const prev = get();
        set(partial, replace);
        const next = get();

        // Detect changed keys and send to Editor
        const allKeys = new Set([...Object.keys(prev), ...Object.keys(next)]);
        for (const key of allKeys) {
          const prevVal = prev[key];
          const nextVal = next[key];
          if (prevVal !== nextVal && typeof nextVal !== "function") {
            bridge.send({
              type: "state-change",
              signalId: `${storeName}.${key}`,
              name: key,
              value: nextVal,
              timestamp: Date.now(),
            });
          }
        }
      };

      return config(trackedSet, get, api);
    };
  };
}

/**
 * Creates a Zustand store with debug tracking middleware.
 * Drop-in replacement for zustand's `create()`.
 */
export function createTrackedStore<T extends Record<string, unknown>>(
  storeName: string,
  creator: StateCreator<T>
) {
  // Use a store name derived creator with middleware
  const withMiddleware: StateCreator<T> = omniDebugMiddleware<T>(storeName)(creator);
  return create<T>()(withMiddleware);
}

// ── Execution Tracing ────────────────────────────────────

/**
 * Mark a node/edge as "executed" for flow highlighting in the Editor.
 * Call this from generated code at the start of action handlers.
 */
export function traceExecution(nodeId: string, edgeId?: string): void {
  bridge.trace(nodeId, edgeId);
}

// ── Exports ──────────────────────────────────────────────

export { bridge as omniRuntime };
