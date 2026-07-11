import { useState, useCallback, useEffect, useRef } from "react";
import { emit, listen } from "@tauri-apps/api/event";
import { api } from "../../commands";

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
  const [isOpen, setIsOpen] = useState(false);
  const nidMapRef = useRef<Record<string, string>>({});

  const open = useCallback(async () => {
    try {
      await api.openDebugWindow();
      setIsOpen(true);
    } catch (e) {
      console.error("[DebugWindow] Failed to open:", e);
    }
  }, []);

  const close = useCallback(async () => {
    try {
      await api.closeDebugWindow();
      setIsOpen(false);
    } catch (e) {
      console.error("[DebugWindow] Failed to close:", e);
    }
  }, []);

  /** Push compiled OmniView output to the debug window. */
  const pushCompileResult = useCallback(async (payload: CompilePayload) => {
    if (!isOpen) return;
    if (payload.nidMap) {
      nidMapRef.current = payload.nidMap;
    }
    try {
      await emit("debug:compile-result", payload);
    } catch (e) {
      console.error("[DebugWindow] Failed to push compile result:", e);
    }
  }, [isOpen]);

  /** Send highlight command to debug window. nid=null to clear. */
  const highlight = useCallback(async (nid: string | null) => {
    if (!isOpen) return;
    try {
      await emit("debug:highlight", { nid });
    } catch (e) {
      console.error("[DebugWindow] Failed to highlight:", e);
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
      // Debug window is ready to receive compile results
      // The next compile cycle will push automatically
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
      // Reverse-lookup: find nid for the given design element ID
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

  return { isOpen, open, close, pushCompileResult, highlight };
}
