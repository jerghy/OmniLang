import { useState, useCallback } from "react";
import { useEditorStore } from "../../store/editor";
import { handleRun as runService, handleStop as stopService } from "../../services/runtime/run-service";

export interface DebugState {
  debugConnected: boolean;
  debugStateChanges: Array<{
    signalId: string;
    name: string;
    value: unknown;
    timestamp: number;
  }>;
  debugBreakpoints: Set<string>;
  activeNodeIds: Set<string>;
  activeEdgeIds: Set<string>;
  debugPaused: boolean;
}

export function useRun(target: string, setStatus: (s: { text: string; isError: boolean }) => void) {
  const setCompileErrors = useEditorStore((s) => s.setCompileErrors);
  const [runStatus, setRunStatus] = useState<{ running: boolean; url: string | null }>({
    running: false,
    url: null,
  });
  const [runLoading, setRunLoading] = useState(false);
  const [fnToNodeMap, setFnToNodeMap] = useState<Record<string, string>>({});
  const [debugState, setDebugState] = useState<DebugState>({
    debugConnected: false,
    debugStateChanges: [],
    debugBreakpoints: new Set(),
    activeNodeIds: new Set(),
    activeEdgeIds: new Set(),
    debugPaused: false,
  });

  const setDebugConnected = useCallback((v: boolean) => {
    setDebugState((prev) => ({ ...prev, debugConnected: v }));
  }, []);
  const setDebugStateChanges = useCallback(
    (v: Array<{ signalId: string; name: string; value: unknown; timestamp: number }> | ((prev: Array<{ signalId: string; name: string; value: unknown; timestamp: number }>) => Array<{ signalId: string; name: string; value: unknown; timestamp: number }>)) => {
      if (typeof v === 'function') {
        setDebugState((prev) => ({ ...prev, debugStateChanges: v(prev.debugStateChanges) }));
      } else {
        setDebugState((prev) => ({ ...prev, debugStateChanges: v }));
      }
    },
    [],
  );
  const setActiveNodeIds = useCallback((v: Set<string> | ((prev: Set<string>) => Set<string>)) => {
    setDebugState((prev) => ({ ...prev, activeNodeIds: typeof v === 'function' ? v(prev.activeNodeIds) : v }));
  }, []);
  const setActiveEdgeIds = useCallback((v: Set<string> | ((prev: Set<string>) => Set<string>)) => {
    setDebugState((prev) => ({ ...prev, activeEdgeIds: typeof v === 'function' ? v(prev.activeEdgeIds) : v }));
  }, []);
  const setDebugPaused = useCallback((v: boolean) => {
    setDebugState((prev) => ({ ...prev, debugPaused: v }));
  }, []);

  const handleRun = useCallback(async () => {
    await runService(target, setStatus, setCompileErrors as (e: unknown[]) => void, setRunLoading, setRunStatus, setFnToNodeMap);
  }, [target, setStatus, setCompileErrors]);

  const handleStop = useCallback(async () => {
    await stopService(
      setStatus,
      setRunStatus,
      setDebugConnected,
      setDebugStateChanges,
      setActiveNodeIds,
      setActiveEdgeIds,
      setDebugPaused,
      setFnToNodeMap,
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps -- all runtime setters are stable useCallback wrappers
  }, [setStatus]);

  const handleClearDebug = useCallback(() => {
    setDebugStateChanges([]);
  }, [setDebugStateChanges]);

  const handleToggleBreakpoint = useCallback((signalId: string) => {
    setDebugState((prev) => {
      const next = new Set(prev.debugBreakpoints);
      if (next.has(signalId)) {
        next.delete(signalId);
        import("../../services/api").then(({ api }) =>
          api.sendDebugCommand({ type: "remove-breakpoint", signalId }),
        );
      } else {
        next.add(signalId);
        import("../../services/api").then(({ api }) =>
          api.sendDebugCommand({ type: "set-breakpoint", signalId }),
        );
      }
      return { ...prev, debugBreakpoints: next };
    });
  }, []);

  const handleStep = useCallback(() => {
    import("../../services/api").then(({ api }) => api.sendDebugCommand({ type: "step" }));
  }, []);

  const handleResume = useCallback(() => {
    import("../../services/api").then(({ api }) => api.sendDebugCommand({ type: "resume" }));
  }, []);

  return {
    runStatus,
    runLoading,
    fnToNodeMap,
    debugState,
    handleRun,
    handleStop,
    handleClearDebug,
    handleToggleBreakpoint,
    handleStep,
    handleResume,
    setFnToNodeMap,
    setDebugConnected,
    setDebugStateChanges,
    setActiveNodeIds,
    setActiveEdgeIds,
    setDebugPaused,
  };
}
