// components/design/OmniViewRenderer.tsx
// WASM-driven renderer — single compilation path, no TS fallback.
// Calls useWasmCompile for debounced WASM compilation, passes results
// to OmniViewElementTree for React rendering and useCssScopePool for CSS.

import { useCallback, useMemo, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { useWasmCompile } from "./useWasmCompile";
import { useCssScopePool } from "./useCssScopePool";
import { OmniViewElementTree } from "./OmniViewElementTree";

export function OmniViewRenderer() {
  const selectedElementIds = usePoiesisStore((s) => s.selectedElementIds);
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);
  const codeFlowVisible = usePoiesisStore((s) => s.designPage.codeFlowVisible);

  const { output, isCompiling, errors } = useWasmCompile();
  const { updateScopes } = useCssScopePool();
  const styleContainerRef = useRef<HTMLDivElement>(null);

  // Update CSS scopes when output changes
  useEffect(() => {
    if (!output) return;
    updateScopes(output.css, output.component_css);
  }, [output, updateScopes]);

  // nid → elementId mapping (from sourceMap or element_classes keys)
  const selectedNid = useMemo(() => {
    if (!output || selectedElementIds.length === 0) return null;
    // element_classes maps element.id → string[]
    // Find the nid that matches the selected element ID
    for (const [id] of Object.entries(output.element_classes)) {
      if (selectedElementIds.includes(id)) {
        // The nid = id (they match in the current WASM; for component expansion,
        // we'll map through sourceMap later)
        return id;
      }
    }
    return null;
  }, [output, selectedElementIds]);

  const handleSelectNid = useCallback(
    (nid: string) => {
      // Map nid → elementId: in current WASM, element_classes keys ARE element IDs
      // For expanded components, the mapping is nid → originalDefId
      selectElement(nid);
    },
    [selectElement],
  );

  const handleDoubleClickNid = useCallback(
    (nid: string) => {
      toggleCodeFlow(codeFlowVisible ? null : nid);
    },
    [toggleCodeFlow, codeFlowVisible],
  );

  return (
    <div
      data-renderer="omniview"
      style={{
        width: "100%", height: "100%", overflow: "auto",
        padding: 8, boxSizing: "border-box", position: "relative",
      }}
    >
      {/* CSS scope pool container */}
      <div ref={styleContainerRef} data-css-scopes="">
        {/* Managed by useCssScopePool via direct DOM */}
      </div>

      {errors.length > 0 && (
        <div style={{
          position: "absolute", bottom: 8, right: 8,
          background: "#fef2f2", border: "1px solid #fecaca",
          borderRadius: 6, padding: "6px 12px", fontSize: 11,
          color: "#dc2626", maxWidth: 320, zIndex: 10,
        }}>
          {errors[0]}
        </div>
      )}

      {isCompiling && (
        <div style={{
          position: "absolute", top: 8, right: 8,
          background: "#eff6ff", border: "1px solid #bfdbfe",
          borderRadius: 4, padding: "2px 8px", fontSize: 11,
          color: "#3b82f6", zIndex: 10,
        }}>
          compiling...
        </div>
      )}

      {output ? (
        <OmniViewElementTree
          output={output}
          selectedNid={selectedNid}
          onSelectNid={handleSelectNid}
          onDoubleClickNid={handleDoubleClickNid}
        />
      ) : (
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "center",
          height: "100%", color: "#94a3b8", fontSize: 13,
        }}>
          空画布 — 从组件面板拖入元素
        </div>
      )}
    </div>
  );
}
