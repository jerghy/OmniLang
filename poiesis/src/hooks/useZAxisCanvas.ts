// hooks/useZAxisCanvas.ts
// Hook managing the Z-axis dual-layer transition state and animation.
// Provides derived state for the canvas layer positions.

import { useMemo } from "react";
import { usePoiesisStore } from "../store";

export interface ZAxisLayout {
  /** Container for the top layer (OmniViewRenderer) */
  topLayerStyle: React.CSSProperties;
  /** Whether the bottom layer (CodeFlowCanvas) is visible */
  bottomVisible: boolean;
  /** Current code flow element ID */
  codeFlowElementId: string | null;
}

export function useZAxisCanvas(): ZAxisLayout {
  const codeFlowVisible = usePoiesisStore(
    (s) => s.designPage.codeFlowVisible
  );
  const codeFlowElementId = usePoiesisStore(
    (s) => s.designPage.codeFlowElementId
  );

  const topLayerStyle = useMemo(
    (): React.CSSProperties => ({
      position: "absolute",
      transition: "all 300ms ease",
      top: 0,
      left: 0,
      right: codeFlowVisible ? "60%" : 0,
      bottom: codeFlowVisible ? "60%" : 0,
      zIndex: 1,
      borderRadius: codeFlowVisible ? 8 : 0,
      boxShadow: codeFlowVisible
        ? "0 4px 16px rgba(0,0,0,0.15)"
        : "none",
      overflow: "auto",
      background: "#fff",
    }),
    [codeFlowVisible]
  );

  return {
    topLayerStyle,
    bottomVisible: codeFlowVisible,
    codeFlowElementId,
  };
}
