import { useCallback, useEffect, useRef, useSyncExternalStore } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { api } from "../../commands";
import { Logger } from "../../lib/utils/logger";

const log = new Logger({ prefix: "DebugWindow" });

// ── Module-level shared state for isOpen ──
// All useDebugWindow() callers share the same isOpen state via useSyncExternalStore.
// This prevents the "DebugWindowButton opens window but useDebugCompile thinks it's closed" bug.

let _isOpen = false;
let _windowReadyCounter = 0;
const _listeners = new Set<() => void>();

// useSyncExternalStore compares snapshots with Object.is.
// Returning a new object every call would cause infinite re-renders
// because Object.is({...}, {...}) is always false.
// Instead, return primitives so Object.is works correctly.
function getIsOpenSnapshot(): boolean {
  return _isOpen;
}

function getWindowReadyCounterSnapshot(): number {
  return _windowReadyCounter;
}

function subscribeDebugWindow(listener: () => void): () => void {
  _listeners.add(listener);
  return () => { _listeners.delete(listener); };
}

function notifyListeners(): void {
  for (const listener of _listeners) {
    listener();
  }
}

function setDebugWindowOpen(value: boolean): void {
  if (_isOpen === value) return;
  _isOpen = value;
  if (!value) _windowReadyCounter = 0; // reset on close
  notifyListeners();
}

function incrementWindowReady(): void {
  _windowReadyCounter++;
  notifyListeners();
}

interface CompilePayload {
  js: string;
  html: string;
  css: string;
  nidMap?: Record<string, string>;
}

interface ElementClickPayload {
  nid: string;
  tag: string;
}

export function useDebugWindow() {
  const isOpen = useSyncExternalStore(subscribeDebugWindow, getIsOpenSnapshot);
  const windowReadyCounter = useSyncExternalStore(subscribeDebugWindow, getWindowReadyCounterSnapshot);
  const nidMapRef = useRef<Record<string, string>>({});

  const open = useCallback(() => {
    setDebugWindowOpen(true);

    api.openDebugWindow()
      .catch((e) => {
        log.error("Failed to open debug window:", e);
        setDebugWindowOpen(false);
      });
  }, []);

  const close = useCallback(() => {
    setDebugWindowOpen(false);

    api.closeDebugWindow()
      .catch((e) => {
        log.error("Failed to close debug window:", e);
      });
  }, []);

  /** Push compiled OmniView output to the debug window (full page replacement). */
  const pushCompileResult = useCallback(async (payload: CompilePayload) => {
    if (!isOpen) return;
    if (payload.nidMap) {
      nidMapRef.current = payload.nidMap;
    }
    try {
      await emit("debug:compile-result", payload);
    } catch (e) {
      log.error("Failed to push compile result:", e);
    }
  }, [isOpen]);

  /** Push per-component JS deltas for hot-replace (no full page reload). */
  const pushHotReplace = useCallback(async (components: Record<string, { js: string }>) => {
    if (!isOpen) return;
    try {
      await emit("debug:hot-replace", { components });
    } catch (e) {
      log.error("Failed to push hot-replace:", e);
    }
  }, [isOpen]);

  /** Send highlight command to debug window. nid=null to clear. */
  const highlight = useCallback(async (nid: string | null) => {
    if (!isOpen) return;
    try {
      await emit("debug:highlight", { nid });
    } catch (e) {
      log.error("Failed to highlight:", e);
    }
  }, [isOpen]);

  // Listen for element clicks from debug window → dispatch poiesis:debug-select
  useEffect(() => {
    const unlistenPromise = listen<ElementClickPayload>("debug:element-click", (event) => {
      const { nid } = event.payload;
      const designId = nidMapRef.current[nid];
      if (designId) {
        window.dispatchEvent(new CustomEvent("poiesis:debug-select", { detail: { nid, designId } }));
      }
    });
    return () => { unlistenPromise.then((fn) => fn()); };
  }, []);

  // Listen for window-ready from debug window
  useEffect(() => {
    const unlistenPromise = listen("debug:window-ready", () => {
      incrementWindowReady();
    });
    return () => { unlistenPromise.then((fn) => fn()); };
  }, []);

  // Listen for editor selection → highlight in debug window
  useEffect(() => {
    const handler = (e: Event) => {
      const { elementId } = (e as CustomEvent).detail;
      if (!elementId) {
        highlight(null);
        return;
      }
      const nidMap = nidMapRef.current;
      for (const [nid, designId] of Object.entries(nidMap)) {
        if (designId === elementId) {
          highlight(nid);
          return;
        }
      }
      highlight(null);
    };
    window.addEventListener("poiesis:editor-select", handler);
    return () => window.removeEventListener("poiesis:editor-select", handler);
  }, [highlight]);

  return { isOpen, windowReadyCounter, open, close, pushCompileResult, pushHotReplace, highlight };
}
