import { useEffect, useRef } from "react";
import { usePoiesisStore } from "../../store";

/**
 * Upper layer: iframe rendering the full app.
 * For Phase 3, implements the container + iframe + basic bridge.
 * Actual WASM compilation will be wired in Phase 4.
 */
export function ArchitecturePreview() {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const selectBubbleByPid = usePoiesisStore((s) => s.selectBubbleByPid);
  const selectedPid = usePoiesisStore((s) => s.architecture.selectedPid);

  // Listen for clicks from iframe
  useEffect(() => {
    const handler = (e: MessageEvent) => {
      if (e.data?.type === "poiesis:element-click" && e.data.pid) {
        selectBubbleByPid(e.data.pid);
      }
    };
    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [selectBubbleByPid]);

  // Send highlight to iframe when selection changes
  useEffect(() => {
    if (selectedPid && iframeRef.current?.contentWindow) {
      iframeRef.current.contentWindow.postMessage(
        { type: "poiesis:highlight", pid: selectedPid },
        window.location.origin
      );
    }
  }, [selectedPid]);

  return (
    <iframe
      ref={iframeRef}
      src="about:blank"
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        background: "var(--ol-surface)",
      }}
      title="App Preview"
      sandbox="allow-scripts"
    />
  );
}
