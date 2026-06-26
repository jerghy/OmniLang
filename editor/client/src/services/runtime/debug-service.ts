// services/runtime/debug-service.ts
// Debug WebSocket message handling — extracted 1:1 from App.tsx handleDebugMessage.

/**
 * Handle an incoming debug WebSocket message.
 * Dispatches on message type to update the appropriate debug state.
 */
export function handleDebugMessage(
  msg: Record<string, unknown>,
  setDebugStateChanges: (updater: (prev: Array<{ signalId: string; name: string; value: unknown; timestamp: number }>) => Array<{ signalId: string; name: string; value: unknown; timestamp: number }>) => void,
  setDebugConnected: (v: boolean) => void,
  setDebugPaused: (v: boolean) => void,
  setActiveNodeIds: (updater: (prev: Set<string>) => Set<string>) => void,
  setActiveEdgeIds: (updater: (prev: Set<string>) => Set<string>) => void,
  setFnToNodeMap: (m: Record<string, string>) => void,
  fnToNodeMapRef: { current: Record<string, string> },
): void {
  const msgType = msg.type as string;
  switch (msgType) {
    case "state-change":
      setDebugStateChanges((prev) => {
        const next = [
          ...prev,
          {
            signalId: (msg.signalId as string) || "",
            name: (msg.name as string) || "",
            value: msg.value,
            timestamp: (msg.timestamp as number) || Date.now(),
          },
        ];
        return next.length > 200 ? next.slice(-200) : next;
      });
      break;

    case "execution-trace": {
      const semanticId = msg.nodeId as string;
      const edgeId = msg.edgeId as string | undefined;
      // Translate semantic ID ("Component.fnName") → canvas element ID
      const nodeId = fnToNodeMapRef.current[semanticId] || semanticId;
      if (nodeId) {
        setActiveNodeIds((prev) => {
          const next = new Set(prev);
          next.add(nodeId);
          setTimeout(() => {
            setActiveNodeIds((p) => {
              const updated = new Set(p);
              updated.delete(nodeId);
              return updated;
            });
          }, 1500);
          return next;
        });
      }
      if (edgeId) {
        setActiveEdgeIds((prev) => {
          const next = new Set(prev);
          next.add(edgeId);
          setTimeout(() => {
            setActiveEdgeIds((p) => {
              const updated = new Set(p);
              updated.delete(edgeId);
              return updated;
            });
          }, 1500);
          return next;
        });
      }
      break;
    }

    case "runtime-ready":
      setDebugConnected(true);
      setDebugStateChanges(() => []);
      break;

    case "paused":
      setDebugPaused(true);
      break;

    case "resumed":
      setDebugPaused(false);
      break;

    case "runtime-disconnected":
      setDebugConnected(false);
      setActiveNodeIds(() => new Set());
      setActiveEdgeIds(() => new Set());
      setDebugPaused(false);
      setFnToNodeMap({});
      break;
  }
}
