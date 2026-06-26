import { useEffect, useCallback, useRef } from "react";
import { api } from "../../services/api";
import { handleDebugMessage as debugMessageService } from "../../services/runtime/debug-service";

export function useDebugSession(
  running: boolean,
  setDebugConnected: (v: boolean) => void,
  setDebugStateChanges: (v: Array<{ signalId: string; name: string; value: unknown; timestamp: number }>) => void,
  setActiveNodeIds: (v: Set<string>) => void,
  setActiveEdgeIds: (v: Set<string>) => void,
  setDebugPaused: (v: boolean) => void,
  setFnToNodeMap: (v: Record<string, string>) => void,
  fnToNodeMap: Record<string, string>,
) {
  const fnToNodeMapRef = useRef(fnToNodeMap);
  fnToNodeMapRef.current = fnToNodeMap;

  const handleDebugMessage = useCallback(
    (msg: Record<string, unknown>) => {
      (debugMessageService as any)(
        msg,
        setDebugStateChanges,
        setDebugConnected,
        setDebugPaused,
        setActiveNodeIds,
        setActiveEdgeIds,
        setFnToNodeMap,
        fnToNodeMapRef,
      );
    },
    [setDebugStateChanges, setDebugConnected, setDebugPaused, setActiveNodeIds, setActiveEdgeIds, setFnToNodeMap],
  );

  useEffect(() => {
    if (running) {
      api.connectDebugWs(handleDebugMessage, setDebugConnected);
      return () => {
        api.disconnectDebug();
        setDebugConnected(false);
      };
    }
  }, [running, handleDebugMessage, setDebugConnected]);
}
