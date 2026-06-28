// LayoutView — Canvas-powered layout canvas.
// Replaces the old mock-DOM rendering with an iframe showing compiled OmniView output.
// Palette -> canvas drops are captured by a transparent overlay on top of the iframe.

import React, { useCallback, useState, useEffect, useRef } from "react";
import { useEditorStore } from "../../../store/editor";
import type { ElementData } from "../../../core/types";
import { useUndoStore } from "../../../store/undo";
import { useCodeGraphStore } from "../../../codegraph/store";
import { AddElement } from "../../../core/commands/AddElement";
import { RemoveElement } from "../../../core/commands/RemoveElement";
import type { CommandContext } from "../../../core/commands/types";
import { uid, extractComponentName } from "../../../core/utils";
import { getDefaultProps } from "../../../core/constants";
import { useComponentRegistry } from "../../../store/componentRegistry";
import { CanvasPreview } from "./CanvasPreview";
import { compilerCache } from "../../../services/compiler/cache";
import { rustCompiler } from "../../../api/rustCompiler";
import type { NidEntry } from "./CanvasPreview";
import { Logger } from "../../../utils/logger";

const log = new Logger({ prefix: "LayoutView" });

// ============================================================
// Component
// ============================================================

interface LayoutViewProps {
  ctx: CommandContext;
  hideCodeBlocks?: boolean;
}

export function LayoutView({ ctx, hideCodeBlocks = false }: LayoutViewProps) {
  const elements = useEditorStore((s) => s.elements);
  const selectedIds = useEditorStore((s) => s.selectedElementIds);
  const selectElement = useEditorStore((s) => s.selectElement);
  const activeDocId = useEditorStore((s) => s.activeDocId);
  const undoExec = useUndoStore((s) => s.execute);

  // -- Compilation state --
  const [srcdoc, setSrcdoc] = useState<string>("");
  const prevDocRef = useRef(activeDocId);
  const [wasmReady, setWasmReady] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const initAttempted = useRef(false);

  // -- Initialize WASM compiler --
  useEffect(() => {
    if (initAttempted.current) return;
    initAttempted.current = true;
    rustCompiler.init().then(() => {
      setWasmReady(true);
      log.info('WASM compiler ready');
    });
  }, []);

  const hasContent = elements.some(
    (e) => !e.parentId && (hideCodeBlocks ? e.nodeType !== "codeblock" : true),
  );

  // -- Recompile helper --
  const recompile = useCallback(() => {
    const store = useEditorStore.getState();
    const cg = useCodeGraphStore.getState();
    const codeTabs = cg.toSerializable().tabs;

    const contentAvailable = store.elements.some(
      (e) =>
        !e.parentId &&
        (hideCodeBlocks ? e.nodeType !== "codeblock" : true),
    );
    if (!contentAvailable) {
      setSrcdoc("");
      return;
    }

    // Determine the correct component name: use the component name when on
    // a component canvas, otherwise extract from source or default to "App".
    // ⚠️ activeFileContent may contain a project-level component name (e.g. "demo1")
    // that doesn't match the style registry key "App". Only use it on file tabs.
    let componentName = "App";
    if (store.activeDocId?.startsWith("comp:")) {
      componentName = store.activeDocId.replace("comp:", "");
    } else if (store.activeDocId?.startsWith("file:") && store.activeFileContent) {
      componentName = extractComponentName(store.activeFileContent, "App");
    }

    const output = compilerCache.compileProject(
      {
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName,
        rootOrder: store.rootOrder,
      },
      true, // editorMode — inject bridge code for postMessage selection
    );

    if (output) {
      setSrcdoc(output.srcdoc);
      // 仅当项目中已有未保存修改（用户做了视觉编辑）时才更新 sourceCode
      // 初始加载时 isDirty==false，不会误触，避免页面显示"未保存"状态
      if (output.omniSource && store.isDirty && output.omniSource !== store.sourceCode) {
        useEditorStore.getState().setSourceCode(output.omniSource);
      }
    }
  }, [hideCodeBlocks]); // recompile helper reads state via useEditorStore.getState()

  // -- Initial compile on mount (re-runs when WASM becomes ready) --
  useEffect(() => {
    if (!wasmReady) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- initial compile when WASM becomes ready
    recompile();
  }, [recompile, wasmReady]);

  // -- Force recompile when activeDocId (canvas tab) changes --
  useEffect(() => {
    if (!wasmReady) return;
    if (prevDocRef.current === activeDocId) return;
    prevDocRef.current = activeDocId;
    recompile();
  }, [activeDocId, wasmReady, recompile]);

  // -- Subscribe to store changes and debounce recompilation --
  // Only recompile when elements, wires, or rootOrder actually change to
  // avoid triggering needless iframe reloads on every UI interaction.
  useEffect(() => {
    let prevKey = "";
    const unsub = useEditorStore.subscribe((state) => {
      // Compute a lightweight key: join element IDs, wire IDs, rootOrder, compVersion,
      // and activeDocId to force recompile on tab/canvas switch.
      const elKey = state.elements.map((e) => `${e.id}:${e.tag}:${e.parentId}:${e.componentRef || ''}:${JSON.stringify(e.style ?? null)}`).join('|');
      const wireKey = state.wires.map((w) => `${w.id}:${w.fromId}->${w.toId}`).join('|');
      const key = `${elKey}|${wireKey}|${state.rootOrder.join(',')}|${state.compVersion}|${state.activeDocId || 'main'}`;
      if (key === prevKey) return; // No meaningful change → skip recompile
      prevKey = key;

      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(recompile, 300);
    });
    return () => {
      unsub();
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [recompile]);

  // -- Listen for srcdoc updates dispatched by PropertyPanel or App-level --
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      if (detail?.srcdoc) {
        setSrcdoc(detail.srcdoc);
      }
    };
    window.addEventListener("omni-srcdoc-update", handler);
    return () => window.removeEventListener("omni-srcdoc-update", handler);
  }, []);

  // -- iframe callbacks --

  const handleSelect = useCallback(
    (_nid: string, entry: NidEntry) => {
      if (entry.elementId) {
        selectElement(entry.elementId);
      }
    },
    [selectElement],
  );

  const handleRendered = useCallback(() => {
    // nidMap is stored in CanvasPreview's ref, accessible for nid -> elementId lookups
    // We don't need the map in this component directly.
  }, []);

  // -- Palette -> canvas drag overlay --

  const [isDragOver, setIsDragOver] = useState(false);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    if (e.dataTransfer.types.includes("application/omni-component")) {
      setIsDragOver(true);
    }
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      const tag = e.dataTransfer.getData("application/omni-component");
      if (!tag) return;

      const props = getDefaultProps(tag);

      const isCustom =
        useComponentRegistry.getState().definitions[tag] !== undefined;
      const el: ElementData = {
        id: uid(tag),
        nodeType: "view",
        tag,
        props,
        children: [],
        parentId: null,
        ...(isCustom ? { componentRef: tag } : {}),
      };
      undoExec(new AddElement(el), ctx);
    },
    [undoExec, ctx],
  );

  // -- Keyboard: Delete / Backspace / Escape --

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.key === "Delete" || e.key === "Backspace") {
        const ids = useEditorStore.getState().selectedElementIds;
        if (ids.length > 0) {
          e.preventDefault();
          for (const id of ids) {
            undoExec(new RemoveElement(id), ctx);
          }
          useEditorStore.getState().selectElement(null);
        }
      }

      if (e.key === "Escape") {
        useEditorStore.getState().selectElement(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [undoExec, ctx]);

  // -- Context Menu --

  const [ctxMenu, setCtxMenu] = useState<{
    x: number;
    y: number;
    elementId: string;
  } | null>(null);

  const closeCtxMenu = useCallback(() => setCtxMenu(null), []);

  const handleContainerContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    // Right-click on empty canvas area — close any open menu
    setCtxMenu(null);
  }, []);

  // -- Render --

  return (
    <div
      style={{ position: "relative", flex: 1, minHeight: 0, overflow: "hidden" }}
      onDragEnter={handleDragEnter}
      onContextMenu={handleContainerContextMenu}
    >
      {hasContent || srcdoc ? (
        <CanvasPreview
          key={activeDocId || "main"}
          srcdoc={srcdoc}
          onSelect={handleSelect}
          onRendered={handleRendered}
          selectedId={selectedIds.length > 0 ? selectedIds[0] : null}
          style={{ width: "100%", height: "100%" }}
        />
      ) : (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "#9ca3af",
            fontSize: 13,
          }}
        >
          <div style={{ textAlign: "center" }}>
            <div style={{ fontSize: 32, marginBottom: 8 }}>&#x1F3A8;</div>
            <div>编译 .omni 或在左侧组件面板拖入组件</div>
            <div style={{ fontSize: 11, marginTop: 4, color: "#d1d5db" }}>
              Ctrl+Enter 编译
            </div>
          </div>
        </div>
      )}

      {/* Drag overlay for palette -> canvas drops */}
      {isDragOver && (
        <div
          onDragOver={(e) => e.preventDefault()}
          onDrop={handleDrop}
          onDragLeave={() => setIsDragOver(false)}
          style={{
            position: "absolute",
            inset: 0,
            zIndex: 10,
            pointerEvents: "auto",
            background: "rgba(79, 70, 229, 0.05)",
            border: "2px dashed #4F46E5",
          }}
        />
      )}

      {/* Context Menu */}
      {ctxMenu && (
        <>
          <div
            onClick={closeCtxMenu}
            style={{
              position: "fixed",
              inset: 0,
              zIndex: 998,
              background: "transparent",
            }}
          />
          <div
            style={{
              position: "fixed",
              left: ctxMenu.x,
              top: ctxMenu.y,
              zIndex: 999,
              background: "#fff",
              border: "1px solid #e5e7eb",
              borderRadius: 8,
              boxShadow: "0 4px 12px rgba(0,0,0,0.15)",
              padding: 4,
              minWidth: 140,
            }}
          >
            <div
              onClick={() => {
                undoExec(new RemoveElement(ctxMenu.elementId), ctx);
                useEditorStore.getState().selectElement(null);
                closeCtxMenu();
              }}
              style={{
                padding: "6px 12px",
                cursor: "pointer",
                borderRadius: 4,
                fontSize: 13,
                color: "#DC2626",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "#FEF2F2";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
              }}
            >
              &#x1F5D1; 删除
            </div>
          </div>
        </>
      )}
    </div>
  );
}

