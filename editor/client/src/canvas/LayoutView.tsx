// LayoutView — Canvas-powered layout canvas.
// Replaces the old mock-DOM rendering with an iframe showing compiled OmniView output.
// Palette -> canvas drops are captured by a transparent overlay on top of the iframe.

import React, { useCallback, useState, useEffect, useRef } from "react";
import { useEditorStore, type ElementData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { useCodeGraphStore } from "../codegraph/store";
import { AddElement } from "../commands/AddElement";
import { RemoveElement } from "../commands/RemoveElement";
import type { CommandContext } from "../commands/types";
import { uid } from "../utils/id";
import { useComponentRegistry } from "../store/componentRegistry";
import { CanvasPreview } from "./CanvasPreview";
import { compilerCache } from "../omni-compiler-cache";
import type { NidEntry } from "./CanvasPreview";

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
  const undoExec = useUndoStore((s) => s.execute);

  // -- Compilation state --
  const [srcdoc, setSrcdoc] = useState<string>("");
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

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

    const output = compilerCache.compileProject(
      {
        designElements: store.elements,
        designWires: store.wires,
        codeTabs,
        componentName: "App",
      },
      true, // editorMode — inject bridge code for postMessage selection
    );

    if (output) {
      setSrcdoc(output.srcdoc);
    }
  }, [hideCodeBlocks]);

  // -- Initial compile on mount --
  useEffect(() => {
    recompile();
  }, [recompile]);

  // -- Subscribe to store changes and debounce recompilation --
  useEffect(() => {
    const unsub = useEditorStore.subscribe(() => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(recompile, 300);
    });
    return () => {
      unsub();
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, [recompile]);

  // -- iframe callbacks --

  const handleSelect = useCallback(
    (_nid: string, entry: NidEntry) => {
      if (entry.elementId) {
        selectElement(entry.elementId);
      }
    },
    [selectElement],
  );

  const handleRendered = useCallback((_map: Record<string, NidEntry>) => {
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

      const props: Record<string, unknown> = {};
      if (tag === "Text") props.label = "Text";
      else if (tag === "Heading") props.label = "Heading";
      else if (tag === "Button") props.label = "Button";
      else if (tag === "Input") props.placeholder = "Input";

      const isCustom =
        useComponentRegistry.getState().definitions[tag] !== undefined;
      const el: ElementData = {
        id: uid(),
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

  const handleContextMenu = useCallback(
    (e: React.MouseEvent, elementId: string) => {
      e.preventDefault();
      e.stopPropagation();
      setCtxMenu({ x: e.clientX, y: e.clientY, elementId });
    },
    [],
  );

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
          srcdoc={srcdoc}
          onSelect={handleSelect}
          onRendered={handleRendered}
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
