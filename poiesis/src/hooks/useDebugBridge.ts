import { useEffect, useCallback, useRef } from "react";

export type BridgeMessage =
  | { type: "poiesis:element-click"; pid: string }
  | { type: "poiesis:highlight"; pid: string }
  | { type: "poiesis:select"; pid: string | null };

interface UseDebugBridgeOptions {
  iframeRef: React.RefObject<HTMLIFrameElement | null>;
  onElementClick?: (pid: string) => void;
}

/**
 * Hook managing the iframe <-> graph bidirectional communication.
 * Uses window.postMessage for cross-origin iframe messaging.
 */
export function useDebugBridge({ iframeRef, onElementClick }: UseDebugBridgeOptions) {
  const handlerRef = useRef(onElementClick);
  useEffect(() => { handlerRef.current = onElementClick; });

  // Listen for messages from iframe — validate origin for security
  useEffect(() => {
    const handler = (e: MessageEvent) => {
      // Only accept messages from same origin (or the iframe's origin in dev)
      if (e.origin !== window.location.origin) return;
      if (e.data?.type === "poiesis:element-click" && e.data.pid) {
        handlerRef.current?.(e.data.pid);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, []);

  const sendToIframe = useCallback(
    (message: BridgeMessage) => {
      if (iframeRef.current?.contentWindow) {
        iframeRef.current.contentWindow.postMessage(message, window.location.origin);
      }
    },
    [iframeRef]
  );

  const highlightElement = useCallback(
    (pid: string) => {
      sendToIframe({ type: "poiesis:highlight", pid });
    },
    [sendToIframe]
  );

  const selectElement = useCallback(
    (pid: string | null) => {
      sendToIframe({ type: "poiesis:select", pid });
    },
    [sendToIframe]
  );

  return {
    sendToIframe,
    highlightElement,
    selectElement,
  };
}
