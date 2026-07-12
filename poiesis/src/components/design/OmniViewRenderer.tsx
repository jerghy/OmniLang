// components/design/OmniViewRenderer.tsx
// WASM-driven renderer — single compilation path, no TS fallback.
// Calls useWasmCompile for debounced WASM compilation, passes results
// to OmniViewElementTree for React rendering and useCssScopePool for CSS.

import { useCallback, useMemo, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { useWasmCompile } from "./useWasmCompile";
import { useDebugCompile } from "../../features/debug/useDebugCompile";
import { useCssScopePool } from "./useCssScopePool";
import { OmniViewElementTree } from "./OmniViewElementTree";
import { _lastClickPos } from "./OmniViewElement";

export function OmniViewRenderer() {
  const selectedElementIds = usePoiesisStore((s) => s.selectedElementIds);
  const selectElement = usePoiesisStore((s) => s.selectElement);
  const switchTab = usePoiesisStore((s) => s.switchTab);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const toggleCodeFlow = usePoiesisStore((s) => s.toggleCodeFlow);

  const { output, isCompiling, errors } = useWasmCompile();
  useDebugCompile();
  const { updateScopes } = useCssScopePool();
  const styleContainerRef = useRef<HTMLDivElement>(null);

  // Sync output to store so FlowCanvas preview node can read it
  const setPreviewOutput = usePoiesisStore((s) => s.setPreviewOutput);
  useEffect(() => {
    setPreviewOutput(output);
  }, [output, setPreviewOutput]);

  // Update CSS scopes when output changes
  useEffect(() => {
    if (!output) return;
    updateScopes(output.css, output.component_css);
  }, [output, updateScopes]);

  // nid ↔ elementId bi-directional mapping from WASM nid_map
  const { selectedNid, nidToElementId } = useMemo(() => {
    if (!output) return { selectedNid: null, nidToElementId: new Map<string, string>() };
    // nid_map: nid → elementId (UUID). Build reverse: elementId → nid.
    // serde_wasm_bindgen serializes HashMap as JS Map, not plain object
    const nidToId: Map<string, string> = output.nid_map instanceof Map
      ? output.nid_map as Map<string, string>
      : new Map<string, string>(Object.entries(output.nid_map));
    const idToNid = new Map<string, string>();
    for (const [nid, id] of nidToId) idToNid.set(id, nid);
    const selId = selectedElementIds.length > 0 ? selectedElementIds[0] : null;
    return { selectedNid: selId ? (idToNid.get(selId) ?? null) : null, nidToElementId: nidToId };
  }, [output, selectedElementIds]);

  const handleSelectNid = useCallback(
    (nid: string) => {
      const elementId = nidToElementId.get(nid) ?? nid;
      selectElement(elementId);
    },
    [selectElement, nidToElementId],
  );


  // Listen for element selection requests from debug window.
  // Finds the component that owns the element and switches to its tab.
  useEffect(() => {
    const handler = (e: Event) => {
      const { designId } = (e as CustomEvent).detail;
      if (!designId) return;

      // Find which component owns this element
      let ownerComponent: string | null = null;
      for (const [compName, compDef] of Object.entries(componentDefs)) {
        if (compDef.elements?.some((el) => el.id === designId)) {
          ownerComponent = compName;
          break;
        }
      }

      // Switch to the component tab if found and not already active
      if (ownerComponent) {
        const tab = openTabs.find((t) => t.componentName === ownerComponent);
        if (tab && tab.id !== activeTabId) {
          switchTab(tab.id);
          // Selection must happen after tab switch — defer to next tick
          setTimeout(() => selectElement(designId), 50);
          return;
        }
      }

      selectElement(designId);
    };
    window.addEventListener("poiesis:debug-select", handler);
    return () => window.removeEventListener("poiesis:debug-select", handler);
  }, [selectElement, switchTab, componentDefs, openTabs, activeTabId]);

  const handleDoubleClickNid = useCallback(
    (nid: string) => {
      const elementId = nidToElementId.get(nid) ?? nid;
      toggleCodeFlow(elementId, { x: _lastClickPos.x, y: _lastClickPos.y });
    },
    [toggleCodeFlow, nidToElementId],
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
